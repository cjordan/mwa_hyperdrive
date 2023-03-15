// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Code to handle direction-independent calibration.
//!
//! This code borrows heavily from Torrance Hodgson's excellent Julia code at
//! <https://github.com/torrance/MWAjl>

mod error;
#[cfg(test)]
pub(crate) mod tests;

pub(crate) use error::DiCalibrateError;

use std::thread;

use crossbeam_channel::{unbounded, Sender};
use crossbeam_utils::atomic::AtomicCell;
use hifitime::Duration;
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use itertools::Itertools;
use log::{debug, info};
use marlu::{
    c64,
    constants::{FREQ_WEIGHT_FACTOR, TIME_WEIGHT_FACTOR},
    math::num_tiles_from_num_cross_correlation_baselines,
    Jones,
};
use ndarray::{iter::AxisIterMut, prelude::*};
use rayon::prelude::*;
use scopeguard::defer_on_unwind;
use vec1::Vec1;

use crate::{
    averaging::{timesteps_to_timeblocks, Chanblock, Timeblock},
    cli::di_calibrate::DiCalParams,
    io::write::{write_vis, VisTimestep},
    math::average_epoch,
    model::{self, ModellerInfo},
    solutions::CalibrationSolutions,
};

pub(crate) struct CalVis {
    /// Visibilites read from input data.
    pub(crate) vis_data_tfb: Array3<Jones<f32>>,

    /// The weights on the visibilites read from input data.
    pub(crate) vis_weights_tfb: Array3<f32>,

    /// Visibilites generated from the sky-model source list.
    pub(crate) vis_model_tfb: Array3<Jones<f32>>,
}

/// For calibration, read in unflagged visibilities and generate sky-model
/// visibilities.
pub(crate) fn get_cal_vis(
    params: &DiCalParams,
    draw_progress_bar: bool,
) -> Result<CalVis, DiCalibrateError> {
    // TODO: Use all fences, not just the first.
    let fence = params.fences.first();

    // Get the time and frequency resolutions once; these functions issue
    // warnings if they have to guess, so doing this once means we aren't
    // issuing too many warnings.
    let obs_context = params.get_obs_context();
    let time_res = obs_context.guess_time_res();
    let freq_res = obs_context.guess_freq_res();

    let vis_shape = (
        params.get_num_timesteps(),
        fence.chanblocks.len(),
        params.get_num_unflagged_cross_baselines(),
    );
    let num_elems = vis_shape.0 * vis_shape.1 * vis_shape.2;
    // We need this many bytes for each of the data and model arrays to do
    // calibration.
    let size = indicatif::HumanBytes((num_elems * std::mem::size_of::<Jones<f32>>()) as u64);
    debug!(
        "Shape of data and model arrays: ({} timesteps, {} channels, {} baselines; {size} each)",
        vis_shape.0, vis_shape.1, vis_shape.2
    );

    macro_rules! fallible_allocator {
        ($default:expr) => {{
            let mut v = Vec::new();
            match v.try_reserve_exact(num_elems) {
                Ok(()) => {
                    v.resize(num_elems, $default);
                    Ok(Array3::from_shape_vec(vis_shape, v).unwrap())
                }
                Err(_) => {
                    // We need this many gibibytes to do calibration (two
                    // visibility arrays and one weights array).
                    let need_gib = indicatif::HumanBytes(
                        (num_elems
                            * (2 * std::mem::size_of::<Jones<f32>>() + std::mem::size_of::<f32>()))
                            as u64,
                    );

                    Err(DiCalibrateError::InsufficientMemory {
                        // Instead of erroring out with how many bytes we need
                        // for the array we just tried to allocate, error out
                        // with how many bytes we need total.
                        need_gib,
                    })
                }
            }
        }};
    }

    debug!("Allocating memory for input data visibilities and model visibilities");
    let mut vis_data_tfb: Array3<Jones<f32>> = fallible_allocator!(Jones::default())?;
    let mut vis_model_tfb: Array3<Jones<f32>> = fallible_allocator!(Jones::default())?;
    let mut vis_weights_tfb: Array3<f32> = fallible_allocator!(0.0)?;

    // Sky-modelling communication channel. Used to tell the model writer when
    // visibilities have been generated and they're ready to be written.
    let (tx_model, rx_model) = unbounded();

    // Progress bars. Courtesy Dev.
    let multi_progress = MultiProgress::with_draw_target(if draw_progress_bar {
        ProgressDrawTarget::stdout()
    } else {
        ProgressDrawTarget::hidden()
    });
    let read_progress = multi_progress.add(
        ProgressBar::new(vis_shape.0 as _)
            .with_style(
                ProgressStyle::default_bar()
                    .template("{msg:17}: [{wide_bar:.blue}] {pos:2}/{len:2} timesteps ({elapsed_precise}<{eta_precise})").unwrap()
                    .progress_chars("=> "),
            )
            .with_position(0)
            .with_message("Reading data"),
    );
    let model_progress = multi_progress.add(
        ProgressBar::new(vis_shape.0 as _)
            .with_style(
                ProgressStyle::default_bar()
                    .template("{msg:17}: [{wide_bar:.blue}] {pos:2}/{len:2} timesteps ({elapsed_precise}<{eta_precise})").unwrap()
                    .progress_chars("=> "),
            )
            .with_position(0)
            .with_message("Sky modelling"),
    );
    // Only add a model writing progress bar if we need it.
    let model_write_progress = params.model_files.as_ref().map(|_| {
        multi_progress.add(
            ProgressBar::new(vis_shape.0 as _)
                .with_style(
                    ProgressStyle::default_bar()
                        .template("{msg:17}: [{wide_bar:.blue}] {pos:2}/{len:2} timeblocks ({elapsed_precise}<{eta_precise})").unwrap()
                        .progress_chars("=> "),
                )
                .with_position(0)
                .with_message("Model writing"),
        )
    });

    // Use a variable to track whether any threads have an issue.
    let error = AtomicCell::new(false);
    info!("Reading input data and sky modelling");
    let scoped_threads_result = thread::scope(|s| {
        // Mutable slices of the "global" arrays. These allow threads to mutate
        // the global arrays in parallel (using the Arc<Mutex<_>> pattern would
        // kill performance here).
        let vis_data_slices = vis_data_tfb.outer_iter_mut();
        let vis_model_slices = vis_model_tfb.outer_iter_mut();
        let vis_weight_slices = vis_weights_tfb.outer_iter_mut();

        // Input visibility-data reading thread.
        let data_handle = s.spawn(|| {
            // If a panic happens, update our atomic error.
            defer_on_unwind! { error.store(true); }
            read_progress.tick();

            let result = read_vis_data(
                params,
                vis_data_slices,
                vis_weight_slices,
                &error,
                read_progress,
            );
            // If the result of reading data was an error, allow the other
            // threads to see this so they can abandon their work early.
            if result.is_err() {
                error.store(true);
            }
            result
        });

        // Sky-model generation thread.
        let model_handle = s.spawn(|| {
            defer_on_unwind! { error.store(true); }
            model_progress.tick();

            let result = model_vis(
                params,
                vis_model_slices,
                time_res,
                freq_res,
                tx_model,
                &error,
                model_progress,
                #[cfg(feature = "cuda")]
                matches!(params.modeller_info, ModellerInfo::Cpu),
            );
            if result.is_err() {
                error.store(true);
            }
            result
        });

        // Model writing thread. If the user hasn't specified to write the model
        // to a file, then this thread just consumes messages from the modeller.
        let writer_handle = s.spawn(|| {
            defer_on_unwind! { error.store(true); }

            // If the user wants the sky model written out, `model_file` is
            // populated.
            if let Some(model_files) = &params.model_files {
                if let Some(pb) = model_write_progress.as_ref() {
                    pb.tick();
                }

                let fine_chan_freqs = obs_context.fine_chan_freqs.mapped_ref(|&f| f as f64);
                let unflagged_baseline_tile_pairs = params
                    .tile_baseline_flags
                    .tile_to_unflagged_cross_baseline_map
                    .keys()
                    .copied()
                    .sorted()
                    .collect::<Vec<_>>();
                // These timeblocks are distinct from `params.timeblocks`; the
                // latter are for calibration time averaging, whereas we want
                // timeblocks for model visibility averaging.
                let timeblocks = timesteps_to_timeblocks(
                    &obs_context.timestamps,
                    params.output_model_time_average_factor,
                    &params.timesteps,
                );

                let result = write_vis(
                    model_files,
                    params.array_position,
                    obs_context.phase_centre,
                    obs_context.pointing_centre,
                    &obs_context.tile_xyzs,
                    &obs_context.tile_names,
                    obs_context.obsid,
                    &obs_context.timestamps,
                    &params.timesteps,
                    &timeblocks,
                    time_res,
                    params.dut1,
                    freq_res,
                    &fine_chan_freqs,
                    &unflagged_baseline_tile_pairs,
                    &params.flagged_fine_chans,
                    params.output_model_time_average_factor,
                    params.output_model_freq_average_factor,
                    params.input_data.get_marlu_mwa_info().as_ref(),
                    rx_model,
                    &error,
                    model_write_progress,
                );
                if result.is_err() {
                    error.store(true);
                }
                match result {
                    // Discard the result string.
                    Ok(_) => Ok(()),
                    Err(e) => Err(DiCalibrateError::from(e)),
                }
            } else {
                // There's no model to write out, but we still need to handle all of the
                // incoming messages.
                for _ in rx_model.iter() {}
                Ok(())
            }
        });

        // Join all thread handles. This propagates any errors and lets us know
        // if any threads panicked, if panics aren't aborting as per the
        // Cargo.toml. (It would be nice to capture the panic information, if
        // it's possible, but I don't know how, so panics are currently
        // aborting.)
        let result = data_handle.join().unwrap();
        let result = result.and_then(|_| model_handle.join().unwrap());
        result.and_then(|_| writer_handle.join().unwrap())
    });

    // Propagate errors.
    scoped_threads_result?;

    debug!("Multiplying visibilities by weights");

    // Multiply the visibilities by the weights (and baseline weights based on
    // UVW cuts). If a weight is negative, it means the corresponding visibility
    // should be flagged, so that visibility is set to 0; this means it does not
    // affect calibration. Not iterating over weights during calibration makes
    // makes calibration run significantly faster.
    vis_data_tfb
        .outer_iter_mut()
        .into_par_iter()
        .zip(vis_model_tfb.outer_iter_mut())
        .zip(vis_weights_tfb.outer_iter())
        .for_each(|((mut vis_data_fb, mut vis_model_fb), vis_weights_fb)| {
            vis_data_fb
                .outer_iter_mut()
                .zip(vis_model_fb.outer_iter_mut())
                .zip(vis_weights_fb.outer_iter())
                .for_each(|((mut vis_data_b, mut vis_model_b), vis_weights_b)| {
                    vis_data_b
                        .iter_mut()
                        .zip(vis_model_b.iter_mut())
                        .zip(vis_weights_b.iter())
                        .zip(params.baseline_weights.iter())
                        .for_each(|(((data, model), &weight), baseline_weight)| {
                            let weight = f64::from(weight) * *baseline_weight;
                            if weight <= 0.0 {
                                *data = Jones::default();
                                *model = Jones::default();
                            } else {
                                *data = Jones::<f32>::from(Jones::<f64>::from(*data) * weight);
                                *model = Jones::<f32>::from(Jones::<f64>::from(*model) * weight);
                            }
                        });
                });
        });

    info!("Finished reading input data and sky modelling");

    Ok(CalVis {
        vis_data_tfb,
        vis_weights_tfb,
        vis_model_tfb,
    })
}

fn read_vis_data(
    params: &DiCalParams,
    vis_data_slices_fb: AxisIterMut<Jones<f32>, Dim<[usize; 2]>>,
    vis_weight_slices_fb: AxisIterMut<f32, Dim<[usize; 2]>>,
    error: &AtomicCell<bool>,
    progress_bar: ProgressBar,
) -> Result<(), DiCalibrateError> {
    for ((&timestep, vis_data_slice_fb), vis_weight_slice_fb) in params
        .timesteps
        .iter()
        .zip(vis_data_slices_fb)
        .zip(vis_weight_slices_fb)
    {
        params.read_crosses(vis_data_slice_fb, vis_weight_slice_fb, timestep)?;

        // Should we continue?
        if error.load() {
            return Ok(());
        }

        progress_bar.inc(1);
    }

    progress_bar.abandon_with_message("Finished reading input data");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn model_vis(
    params: &DiCalParams,
    vis_model_slices_fb: AxisIterMut<Jones<f32>, Dim<[usize; 2]>>,
    time_res: Duration,
    freq_res: f64,
    tx_model: Sender<VisTimestep>,
    error: &AtomicCell<bool>,
    progress_bar: ProgressBar,
    #[cfg(feature = "cuda")] use_cpu_for_modelling: bool,
) -> Result<(), DiCalibrateError> {
    let obs_context = params.get_obs_context();
    let modeller = model::new_sky_modeller(
        #[cfg(feature = "cuda")]
        use_cpu_for_modelling,
        &*params.beam,
        &params.source_list,
        &params.unflagged_tile_xyzs,
        &params.unflagged_fine_chan_freqs,
        &params.tile_baseline_flags.flagged_tiles,
        obs_context.phase_centre,
        params.array_position.longitude_rad,
        params.array_position.latitude_rad,
        params.dut1,
        params.apply_precession,
    )?;

    let weight_factor =
        ((freq_res / FREQ_WEIGHT_FACTOR) * (time_res.to_seconds() / TIME_WEIGHT_FACTOR)) as f32;

    // Iterate over all calibration timesteps and write to the model slices.
    for (&timestep, mut vis_model_slice) in params.timesteps.iter().zip(vis_model_slices_fb) {
        // If for some reason the timestamp isn't there for this timestep, a
        // programmer stuffed up, but emit a decent error message.
        let timestamp = obs_context
            .timestamps
            .get(timestep)
            .ok_or(DiCalibrateError::TimestepUnavailable { timestep })?;
        match modeller.model_timestep_with(*timestamp, vis_model_slice.view_mut()) {
            // Send the model information to the writer.
            Ok(_) => match tx_model.send(VisTimestep {
                cross_data_fb: vis_model_slice.to_shared(),
                cross_weights_fb: ArcArray::from_elem(vis_model_slice.dim(), weight_factor),
                autos: None,
                timestamp: *timestamp,
            }) {
                Ok(()) => (),
                // If we can't send the message, it's because the channel has
                // been closed on the other side. That should only happen
                // because the writer has exited due to error; in that case,
                // just exit this thread.
                Err(_) => return Ok(()),
            },
            Err(e) => return Err(DiCalibrateError::from(e)),
        }

        // Should we continue?
        if error.load() {
            return Ok(());
        }

        progress_bar.inc(1);
    }

    progress_bar.abandon_with_message("Finished generating sky model");
    Ok(())
}

/// (Possibly) incomplete calibration solutions.
///
/// hyperdrive only reads in the data it needs for DI calibration; it ignores
/// any flagged tiles and channels in the input data. Consequentially, when DI
/// solutions are made from only unflagged data, these incomplete solutions need
/// to be "padded" with NaNs such that the complete calibration solutions can be
/// saved to disk or applied to an observation.
///
/// The struct members here are kind of arbitrary, but they've been chosen
/// because all of them are necessary for `calibrate_timeblocks`, which is the
/// only function to create `IncompleteSolutions`. To "complete" the solutions,
/// extra metadata may be supplied.
pub struct IncompleteSolutions<'a> {
    /// Direction-independent calibration solutions *for only unflagged data*.
    /// The first dimension is timeblock, the second is unflagged tile, the
    /// third is unflagged chanblock.
    pub(crate) di_jones: Array3<Jones<f64>>,

    /// The timeblocks used in calibration.
    timeblocks: &'a Vec1<Timeblock>,

    /// The unflagged chanblocks used in calibration.
    chanblocks: &'a [Chanblock],

    /// The maximum allowed number of iterations during calibration.
    max_iterations: u32,

    /// The stop threshold used during calibration.
    stop_threshold: f64,

    /// The minimum threshold used during calibration.
    min_threshold: f64,
}

impl<'a> IncompleteSolutions<'a> {
    /// Convert these [`IncompleteSolutions`] into "padded"
    /// [`CalibrationSolutions`].
    ///
    /// * `total_num_tiles` is the total number of tiles (including flagged
    ///   tiles).
    /// * `tile_flags` and `flagged_chanblock_indices` are the flagged tile and
    ///   chanblock indices, respectively.
    ///
    /// The remaining arguments are optional and if provided can be written to
    /// output calibration solutions.
    ///
    /// * `obsid` is the observation ID.
    /// * `raw_data_corrections` are not needed but are useful to declare.
    /// * `tile_names` are the tile names of *all* tiles in the array, not just
    ///   unflagged ones.
    /// * `calibration_results` are the precisions that each unflagged
    ///   calibration chanblock converged with. The first dimension is
    ///   timeblock, the second is chanblock.
    /// * `baseline_weights` are the unflagged baseline weights used in
    ///   calibration.
    /// * `uvw_min` and `uvw_max` are the baseline cutoffs used in calibration
    ///   \[metres\].
    /// * `freq_centroid` is the centroid frequency used to convert UVW cutoffs
    ///   in lambdas to metres \[Hz\].
    pub(crate) fn into_cal_sols(
        self,
        params: &DiCalParams,
        calibration_results: Option<Array2<f64>>,
    ) -> CalibrationSolutions {
        let Self {
            di_jones,
            timeblocks,
            chanblocks,
            max_iterations,
            stop_threshold,
            min_threshold,
        } = self;

        let obs_context = params.get_obs_context();
        let total_num_tiles = params.get_total_num_tiles();
        // TODO: Picket fences.
        let flagged_chanblock_indices = &params.fences.first().flagged_chanblock_indices;
        // TODO: Don't use the obs_context here. This needs to be the centroid
        // frequencies of the chanblocks. This only works because frequency
        // averaging (i.e. more than one channel per chanblock) isn't possible
        // right now.
        let chanblock_freqs = obs_context.fine_chan_freqs.mapped_ref(|&u| u as f64);

        let (num_timeblocks, num_unflagged_tiles, num_unflagged_chanblocks) = di_jones.dim();
        let total_num_chanblocks = chanblocks.len() + flagged_chanblock_indices.len();

        // These things should always be true; if they aren't, it's a
        // programmer's fault.
        assert!(!timeblocks.is_empty());
        assert_eq!(num_timeblocks, timeblocks.len());
        assert!(num_unflagged_tiles <= total_num_tiles);
        assert_eq!(
            num_unflagged_tiles + params.tile_baseline_flags.flagged_tiles.len(),
            total_num_tiles
        );
        assert_eq!(num_unflagged_chanblocks, chanblocks.len());
        assert_eq!(
            num_unflagged_chanblocks + flagged_chanblock_indices.len(),
            total_num_chanblocks
        );

        // `out_di_jones` will contain the "complete" calibration solutions. The
        // data is the same as `di_jones`, but NaNs will be placed anywhere
        // a tile or chanblock was flagged. The "chanblock" terminology is
        // deliberate here; the amount of frequency/channel averaging on `self`
        // must propagate to `out_di_jones`.
        let mut out_di_jones = Array3::from_elem(
            (num_timeblocks, total_num_tiles, total_num_chanblocks),
            Jones::from([c64::new(f64::NAN, f64::NAN); 4]),
        );

        // Populate out_di_jones. The timeblocks are always 1-to-1.
        out_di_jones
            .outer_iter_mut()
            .into_par_iter()
            .zip(di_jones.outer_iter())
            .for_each(|(mut out_di_jones, di_jones)| {
                // Iterate over the tiles.
                let mut i_unflagged_tile = 0;
                out_di_jones
                    .outer_iter_mut()
                    .enumerate()
                    .for_each(|(i_tile, mut out_di_jones)| {
                        // Nothing needs to be done if this tile is flagged.
                        if !params.tile_baseline_flags.flagged_tiles.contains(&i_tile) {
                            // Iterate over the chanblocks.
                            let mut i_unflagged_chanblock = 0;
                            out_di_jones.iter_mut().enumerate().for_each(
                                |(i_chanblock, out_di_jones)| {
                                    // Nothing needs to be done if this
                                    // chanblock is flagged.
                                    if !flagged_chanblock_indices.contains(&(i_chanblock as u16)) {
                                        // The incomplete solutions aren't
                                        // inverted (i.e. they go from model to
                                        // data, but we want to store data to
                                        // model).
                                        *out_di_jones = di_jones
                                            [(i_unflagged_tile, i_unflagged_chanblock)]
                                            .inv();
                                        i_unflagged_chanblock += 1;
                                    }
                                },
                            );

                            i_unflagged_tile += 1;
                        }
                    });
            });

        // Include flagged chanblock precisions as NaNs.
        let calibration_results = match calibration_results {
            Some(calibration_results) => {
                let total_num_chanblocks = out_di_jones.len_of(Axis(2));
                let mut out = Array2::from_elem(
                    (out_di_jones.len_of(Axis(0)), total_num_chanblocks),
                    f64::NAN,
                );
                let mut i_unflagged_chanblock = 0;
                for i_chanblock in 0..total_num_chanblocks {
                    if flagged_chanblock_indices.contains(&(i_chanblock as u16)) {
                        continue;
                    } else {
                        out.slice_mut(s![.., i_chanblock])
                            .assign(&calibration_results.slice(s![.., i_unflagged_chanblock]));
                        i_unflagged_chanblock += 1;
                    }
                }
                Some(out)
            }
            None => None,
        };

        // Include flagged baselines as NaNs.
        let baseline_weights = {
            let total_num_baselines = (total_num_tiles * (total_num_tiles - 1)) / 2;
            let mut out = vec![f64::NAN; total_num_baselines];
            let mut i_unflagged_baseline = 0;
            let mut i_baseline = 0;
            for i_tile_1 in 0..total_num_tiles {
                for i_tile_2 in i_tile_1 + 1..total_num_tiles {
                    if params.tile_baseline_flags.flagged_tiles.contains(&i_tile_1)
                        || params.tile_baseline_flags.flagged_tiles.contains(&i_tile_2)
                    {
                        i_baseline += 1;
                        continue;
                    } else {
                        out[i_baseline] = params.baseline_weights[i_unflagged_baseline];
                        i_baseline += 1;
                        i_unflagged_baseline += 1;
                    }
                }
            }
            Vec1::try_from_vec(out).ok()
        };

        CalibrationSolutions {
            di_jones: out_di_jones,
            flagged_tiles: params
                .tile_baseline_flags
                .flagged_tiles
                .iter()
                .copied()
                .sorted()
                .collect(),
            flagged_chanblocks: flagged_chanblock_indices.clone(),
            chanblock_freqs: Some(chanblock_freqs),
            obsid: obs_context.obsid,
            start_timestamps: Some(timeblocks.mapped_ref(|tb| *tb.timestamps.first())),
            end_timestamps: Some(timeblocks.mapped_ref(|tb| *tb.timestamps.last())),
            average_timestamps: Some(timeblocks.mapped_ref(|tb| average_epoch(&tb.timestamps))),
            max_iterations: Some(max_iterations),
            stop_threshold: Some(stop_threshold),
            min_threshold: Some(min_threshold),
            raw_data_corrections: params.raw_data_corrections,
            tile_names: Some(obs_context.tile_names.clone()),
            dipole_gains: Some(params.beam.get_dipole_gains()),
            dipole_delays: params.beam.get_dipole_delays(),
            beam_file: params.beam.get_beam_file().map(|p| p.to_path_buf()),
            calibration_results,
            baseline_weights,
            uvw_min: Some(params.uvw_min),
            uvw_max: Some(params.uvw_max),
            freq_centroid: Some(params.freq_centroid),
            modeller: match &params.modeller_info {
                ModellerInfo::Cpu => Some("CPU".to_string()),

                #[cfg(feature = "cuda")]
                ModellerInfo::Cuda {
                    device_info,
                    driver_info,
                } => Some(format!(
                    "{} (capability {}, {} MiB), CUDA driver {}, runtime {}",
                    device_info.name,
                    device_info.capability,
                    device_info.total_global_mem,
                    driver_info.driver_version,
                    driver_info.runtime_version
                )),
            },
        }
    }
}

/// Perform DI calibration on the data and model. Incomplete DI solutions are
/// returned; these need to be "padded" with NaNs by `into_cal_sols` before they
/// can be saved to disk or applied to an observation's visibilities.
///
/// This function basically wraps `calibrate_timeblock`, which does work in
/// parallel. For this reason, `calibrate_timeblocks` does nothing in parallel.
///
/// The way this code is currently structured mandates that all timesteps are
/// calibrated together (as if they all belonged to a single timeblock) before
/// any timeblocks are individually calibrated. This decision can be revisited.
#[allow(clippy::too_many_arguments)]
pub fn calibrate_timeblocks<'a>(
    vis_data_tfb: ArrayView3<Jones<f32>>,
    vis_model_tfb: ArrayView3<Jones<f32>>,
    timeblocks: &'a Vec1<Timeblock>,
    chanblocks: &'a [Chanblock],
    max_iterations: u32,
    stop_threshold: f64,
    min_threshold: f64,
    draw_progress_bar: bool,
    print_convergence_messages: bool,
) -> (IncompleteSolutions<'a>, Array2<CalibrationResult>) {
    let num_unflagged_tiles = num_tiles_from_num_cross_correlation_baselines(vis_data_tfb.dim().2);
    let num_timeblocks = timeblocks.len();
    let num_chanblocks = chanblocks.len();
    let shape = (num_timeblocks, num_unflagged_tiles, num_chanblocks);
    let mut di_jones = Array3::from_elem(shape, Jones::identity());

    let cal_results = if num_timeblocks == 1 {
        // Calibrate all timesteps together.
        let pb = make_calibration_progress_bar(
            num_chanblocks,
            "Calibrating".to_string(),
            draw_progress_bar,
        );
        let cal_results = calibrate_timeblock(
            vis_data_tfb.view(),
            vis_model_tfb.view(),
            di_jones.view_mut(),
            timeblocks.first(),
            chanblocks,
            max_iterations,
            stop_threshold,
            min_threshold,
            pb,
            print_convergence_messages,
        );
        let total_converged_count = cal_results.iter().filter(|r| r.converged).count();
        info!(
            "All timesteps: {}/{} ({}%) chanblocks converged",
            total_converged_count,
            num_chanblocks,
            ((total_converged_count as f64 / num_chanblocks as f64) * 100.0).round()
        );
        Array2::from_shape_vec((num_timeblocks, num_chanblocks), cal_results).unwrap()
    } else {
        // Calibrate all timesteps together to get a good initial guess at what
        // the solutions for each timeblock should be.
        let pb = make_calibration_progress_bar(
            num_chanblocks,
            "Calibrating all timeblocks together".to_string(),
            draw_progress_bar,
        );
        // This timeblock represents all timeblocks.
        let timeblock = {
            let mut timeblock = timeblocks.first().clone();
            for tb in timeblocks.iter().skip(1) {
                timeblock.range = timeblock.range.start..tb.range.end;
                timeblock.timestamps.extend(tb.timestamps.iter());
            }
            timeblock
        };
        let cal_results = calibrate_timeblock(
            vis_data_tfb.view(),
            vis_model_tfb.view(),
            di_jones.view_mut(),
            &timeblock,
            chanblocks,
            max_iterations,
            stop_threshold,
            min_threshold,
            pb,
            print_convergence_messages,
        );
        let total_converged_count = cal_results.into_iter().filter(|r| r.converged).count();
        info!(
            "All timesteps for initial guesses: {}/{} ({}%) chanblocks converged",
            total_converged_count,
            num_chanblocks,
            ((total_converged_count as f64 / num_chanblocks as f64) * 100.0).round()
        );

        // Calibrate each timeblock individually. Set all solutions to be that
        // of the averaged solutions so that the individual timeblocks have less
        // work to do.
        di_jones.accumulate_axis_inplace(Axis(0), |&prev, curr| *curr = prev);
        let mut all_cal_results = Vec::with_capacity(timeblocks.len());
        for (i_timeblock, timeblock) in timeblocks.iter().enumerate() {
            let pb = make_calibration_progress_bar(
                num_chanblocks,
                format!(
                    "Calibrating timeblock {}/{}",
                    i_timeblock + 1,
                    num_timeblocks
                ),
                draw_progress_bar,
            );
            let mut cal_results = calibrate_timeblock(
                vis_data_tfb.view(),
                vis_model_tfb.view(),
                di_jones.view_mut(),
                timeblock,
                chanblocks,
                max_iterations,
                stop_threshold,
                min_threshold,
                pb,
                print_convergence_messages,
            );
            let total_converged_count = cal_results.iter().filter(|r| r.converged).count();
            info!(
                "Timeblock {}: {}/{} ({}%) chanblocks converged",
                i_timeblock + 1,
                total_converged_count,
                num_chanblocks,
                (total_converged_count as f64 / num_chanblocks as f64 * 100.0).round()
            );
            all_cal_results.append(&mut cal_results);
        }
        Array2::from_shape_vec((num_timeblocks, num_chanblocks), all_cal_results).unwrap()
    };

    (
        IncompleteSolutions {
            di_jones,
            timeblocks,
            chanblocks,
            max_iterations,
            stop_threshold,
            min_threshold,
        },
        cal_results,
    )
}

/// Convenience function to make a progress bar while calibrating. `draw`
/// determines if the progress bar is actually displayed.
fn make_calibration_progress_bar(
    num_chanblocks: usize,
    message: String,
    draw: bool,
) -> ProgressBar {
    ProgressBar::with_draw_target(
        Some(num_chanblocks as _),
        if draw {
            // Use stdout, not stderr, because the messages printed by the
            // progress bar are valuable.
            ProgressDrawTarget::stdout()
        } else {
            ProgressDrawTarget::hidden()
        },
    )
    .with_style(
        ProgressStyle::default_bar()
            .template("{msg}: [{wide_bar:.blue}] {pos:3}/{len:3} ({elapsed_precise}<{eta_precise})")
            .unwrap()
            .progress_chars("=> "),
    )
    .with_position(0)
    .with_message(message)
}

/// Worker function to do calibration.
#[allow(clippy::too_many_arguments)]
fn calibrate_timeblock(
    vis_data_tfb: ArrayView3<Jones<f32>>,
    vis_model_tfb: ArrayView3<Jones<f32>>,
    mut di_jones: ArrayViewMut3<Jones<f64>>,
    timeblock: &Timeblock,
    chanblocks: &[Chanblock],
    max_iterations: u32,
    stop_threshold: f64,
    min_threshold: f64,
    progress_bar: ProgressBar,
    print_convergence_messages: bool,
) -> Vec<CalibrationResult> {
    let (_, num_unflagged_tiles, num_chanblocks) = di_jones.dim();

    let mut di_jones_rev = di_jones
        .slice_mut(s![timeblock.index, .., ..])
        .reversed_axes();

    let mut timeblock_cal_results: Vec<CalibrationResult> = chanblocks
        .par_iter()
        .zip(di_jones_rev.outer_iter_mut())
        .map(|(chanblock, di_jones)| {
            let i_chanblock = chanblock.unflagged_index as usize;
            let range = s![
                timeblock.range.clone(),
                // We use a range because `calibrate` and `calibration_loop`
                // expect visibility arrays with potentially multiple
                // chanblocks. It may be worth enforcing only single chanblocks.
                i_chanblock..i_chanblock + 1,
                ..
            ];
            let mut cal_result = calibrate(
                vis_data_tfb.slice(range),
                vis_model_tfb.slice(range),
                di_jones,
                max_iterations,
                stop_threshold,
                min_threshold,
            );
            cal_result.chanblock = Some(chanblock.chanblock_index as usize);
            cal_result.i_chanblock = Some(chanblock.unflagged_index as usize);

            let mut status_str = format!("Chanblock {:>3}", chanblock.chanblock_index);
            if num_unflagged_tiles - cal_result.num_failed <= 4 {
                status_str.push_str(&format!(
                    ": failed    ({:>2}): Too many antenna solutions failed ({})",
                    cal_result.num_iterations, cal_result.num_failed
                ));
            } else if cal_result.max_precision > min_threshold {
                status_str.push_str(&format!(
                    ": failed    ({:>2}): {:.5e} > {:e}",
                    cal_result.num_iterations, cal_result.max_precision, min_threshold,
                ));
            } else if cal_result.max_precision > stop_threshold {
                status_str.push_str(&format!(
                    ": converged ({:>2}): {:e} > {:.5e} > {:e}",
                    cal_result.num_iterations,
                    min_threshold,
                    cal_result.max_precision,
                    stop_threshold
                ));
            } else {
                status_str.push_str(&format!(
                    ": converged ({:>2}): {:e} > {:.5e}",
                    cal_result.num_iterations, stop_threshold, cal_result.max_precision
                ));
            }
            progress_bar.inc(1);
            if print_convergence_messages {
                if progress_bar.is_hidden() {
                    println!("{status_str}");
                } else {
                    progress_bar.println(status_str);
                }
            }
            cal_result
        })
        .collect();
    assert_eq!(timeblock_cal_results.len(), num_chanblocks);
    let mut total_converged_count = timeblock_cal_results
        .iter()
        .filter(|result| result.converged)
        .count();

    // Attempt to calibrate any chanblocks that failed by taking solutions
    // from nearby chanblocks as starting points.
    if total_converged_count > 0 && total_converged_count != num_chanblocks {
        let mut new_converged_count = 1;

        let mut retry_iter = 0;
        while new_converged_count > 0 && total_converged_count != num_chanblocks {
            retry_iter += 1;
            if print_convergence_messages {
                let s = format!("*** Re-calibrating failed chanblocks iteration {retry_iter} ***");
                if progress_bar.is_hidden() {
                    println!("{s}");
                } else {
                    progress_bar.println(s);
                }
            }

            // Iterate over all the calibration results until we find one
            // that failed. Then find the next that succeeded. With a
            // converged solution on both sides (or either side) of the
            // failures, use a weighted average for a guess of what the
            // Jones matrices should be, then re-run MitchCal.
            let mut left = None;
            let mut pairs = vec![];
            let mut in_failures = false;
            for cal_result in timeblock_cal_results.iter() {
                match (in_failures, cal_result.converged) {
                    (false, true) => left = Some(cal_result.i_chanblock.unwrap()),
                    (false, false) => in_failures = true,
                    (true, true) => {
                        in_failures = false;
                        pairs.push((left, Some(cal_result.i_chanblock.unwrap())));
                        left = Some(cal_result.i_chanblock.unwrap());
                    }
                    (true, false) => (),
                }
            }

            for p in pairs {
                match p {
                    (Some(l), Some(r)) => {
                        let left_sol = di_jones_rev.slice(s![l, ..]).to_owned();
                        let right_sol = di_jones_rev.slice(s![r, ..]).to_owned();
                        for i in l + 1..r {
                            let left_weight = (r - i) as f64;
                            let right_weight = (i - l) as f64;
                            let weighted_sol = (&left_sol * left_weight
                                + &right_sol * right_weight)
                                / (r - l) as f64;
                            di_jones_rev.slice_mut(s![i, ..]).assign(&weighted_sol);
                        }
                    }
                    (Some(l), None) => {
                        let left_sol = di_jones_rev.slice(s![l, ..]).to_owned();
                        di_jones_rev
                            .slice_mut(s![l + 1..num_chanblocks, ..])
                            .assign(&left_sol);
                    }
                    (None, Some(r)) => {
                        let right_sol = di_jones_rev.slice(s![r, ..]).to_owned();
                        di_jones_rev.slice_mut(s![0..r, ..]).assign(&right_sol);
                    }
                    (None, None) => unreachable!(),
                }
            }

            // Repeat calibration on failed chanblocks. Iterate over all
            // failed chanblocks again if any chanblocks managed to
            // converge.
            timeblock_cal_results
                .par_iter_mut()
                .zip(di_jones_rev.outer_iter_mut())
                .for_each(|(old_cal_result, di_jones)| {
                    if !old_cal_result.converged {
                        let chanblock = old_cal_result.chanblock.unwrap();
                        let i_chanblock = old_cal_result.i_chanblock.unwrap();
                        let range = s![timeblock.range.clone(), i_chanblock..i_chanblock + 1, ..];
                        let mut new_cal_result = calibrate(
                            vis_data_tfb.slice(range),
                            vis_model_tfb.slice(range),
                            di_jones,
                            max_iterations,
                            stop_threshold,
                            min_threshold,
                        );
                        new_cal_result.chanblock = Some(chanblock);
                        new_cal_result.i_chanblock = Some(i_chanblock);

                        let mut status_str = format!("Chanblock {chanblock:>3}");
                        if num_unflagged_tiles - new_cal_result.num_failed <= 4 {
                            status_str.push_str(&format!(
                                ": failed    ({:>2}): Too many antenna solutions failed ({})",
                                new_cal_result.num_iterations, new_cal_result.num_failed
                            ));
                        } else if new_cal_result.max_precision > min_threshold {
                            status_str.push_str(&format!(
                                ": failed    ({:>2}): {:.5e} > {:e}",
                                new_cal_result.num_iterations,
                                new_cal_result.max_precision,
                                min_threshold,
                            ));
                        } else if new_cal_result.max_precision > stop_threshold {
                            status_str.push_str(&format!(
                                ": converged ({:>2}): {:e} > {:.5e} > {:e}",
                                new_cal_result.num_iterations,
                                min_threshold,
                                new_cal_result.max_precision,
                                stop_threshold
                            ));
                        } else {
                            status_str.push_str(&format!(
                                ": converged ({:>2}): {:e} > {:.5e}",
                                new_cal_result.num_iterations,
                                stop_threshold,
                                new_cal_result.max_precision
                            ));
                        }
                        if print_convergence_messages {
                            if progress_bar.is_hidden() {
                                println!("{status_str}");
                            } else {
                                progress_bar.println(status_str);
                            }
                        }

                        // Convert precisions that have extremely large exponents to NaN.
                        if new_cal_result.max_precision.abs() > 1e100 {
                            new_cal_result.max_precision = f64::NAN
                        }

                        *old_cal_result = new_cal_result;
                    }
                });
            let tmp = timeblock_cal_results
                .iter()
                .filter(|result| result.converged)
                .count();
            new_converged_count = tmp - total_converged_count;
            total_converged_count = tmp;
        }
    }
    progress_bar.abandon();

    timeblock_cal_results
}

#[derive(Debug)]
pub struct CalibrationResult {
    pub(crate) num_iterations: u32,
    pub(crate) converged: bool,
    pub(crate) max_precision: f64,
    pub(crate) num_failed: usize,
    pub(crate) chanblock: Option<usize>,

    /// The unflagged index of the chanblock. e.g. If there are 10 chanblocks
    /// that *could* be calibrated, but we calibrate only 2-9 (i.e. 0 and 1 are
    /// flagged), then the first chanblock index is 2, but its i_chanblock is 0.
    pub(crate) i_chanblock: Option<usize>,
}

/// Calibrate the antennas of the array by comparing the observed input data
/// against our generated model. Return information on this process in a
/// [`CalibrationResult`].
///
/// This function is intended to be run in parallel; for that reason, no
/// parallel code is inside this function.
pub(super) fn calibrate(
    data_tfb: ArrayView3<Jones<f32>>,
    model_tfb: ArrayView3<Jones<f32>>,
    mut di_jones: ArrayViewMut1<Jones<f64>>,
    max_iterations: u32,
    stop_threshold: f64,
    min_threshold: f64,
) -> CalibrationResult {
    assert_eq!(data_tfb.dim(), model_tfb.dim());
    let num_tiles = di_jones.len_of(Axis(0));

    let mut old_jones: Array1<Jones<f64>> = Array::zeros(di_jones.dim());
    let mut top: Array1<Jones<f64>> = Array::zeros(di_jones.dim());
    let mut bot: Array1<Jones<f64>> = Array::zeros(di_jones.dim());
    // The convergence precisions per antenna. They are stored per polarisation
    // for programming convenience, but really only we're interested in the
    // largest value in the entire array.
    let mut precisions: Array2<f64> = Array::zeros((num_tiles, 4));
    let mut failed: Array1<bool> = Array1::from_elem(num_tiles, false);

    let mut iteration = 0;
    while iteration < max_iterations {
        iteration += 1;
        // Re-initialise top and bot.
        top.fill(Jones::default());
        bot.fill(Jones::default());

        calibration_loop(
            data_tfb,
            model_tfb,
            di_jones.view(),
            top.view_mut(),
            bot.view_mut(),
        );

        // Do a once-off check to see if `top` and `bot` are already identical.
        // This occurs on flagged channels containing no data, or perhaps with
        // simulated data.
        if iteration == 1 {
            let mut top_and_bot_are_equal = true;
            let mut top_and_bot_are_zeros = true;
            for (top, bot) in top.iter().zip(bot.iter()) {
                let diff = *top - bot;
                for diff in diff.to_float_array() {
                    if diff.abs() > stop_threshold {
                        top_and_bot_are_equal = false;
                        top_and_bot_are_zeros = false;
                        break;
                    }
                }
                if !top_and_bot_are_equal && !top_and_bot_are_zeros {
                    break;
                }

                if top_and_bot_are_zeros {
                    for top in top.to_float_array() {
                        if top.abs() > f64::EPSILON {
                            top_and_bot_are_zeros = false;
                            break;
                        }
                    }
                }
                if top_and_bot_are_zeros {
                    for bot in bot.to_float_array() {
                        if bot.abs() > f64::EPSILON {
                            top_and_bot_are_zeros = false;
                            break;
                        }
                    }
                }
                if !top_and_bot_are_equal && !top_and_bot_are_zeros {
                    break;
                }
            }

            // `top` and `bot` are the same; this implies that there's no need
            // for calibration.
            if top_and_bot_are_equal {
                // Rather than simply continuing and allowing the calibration
                // solutions to be identity, check if `top` and `bot` are zeros;
                // in this case, we assume that this channel had no data, so we
                // mark all tiles as failed.
                if top_and_bot_are_zeros {
                    failed.fill(true);
                }
                break;
            }
        }

        // Obtain the new DI Jones matrices from "top" and "bot".
        // Tile/antenna axis.
        di_jones
            .outer_iter_mut()
            .zip(old_jones.outer_iter_mut())
            .zip(top.outer_iter())
            .zip(bot.outer_iter())
            .zip(failed.iter_mut())
            .filter(|(_, &mut failed)| !failed)
            .for_each(|((((mut di_jones, mut old_jones), top), bot), failed)| {
                // Unflagged fine-channel axis.
                di_jones
                    .iter_mut()
                    .zip(old_jones.iter_mut())
                    .zip(top.iter())
                    .zip(bot.iter())
                    .for_each(|(((di_jones, old_jones), top), bot)| {
                        let div = *top / bot;
                        if div.any_nan() {
                            *failed = true;
                            *di_jones = Jones::default();
                            *old_jones = Jones::default();
                        } else {
                            *di_jones = div;
                        }
                    });
            });

        // More than 4 antenna need to be present to get a good solution.
        let num_failed = failed.iter().filter(|&&f| f).count();
        if num_tiles - num_failed <= 4 {
            break;
        }

        // On every even iteration, we test for convergence and set the new gain
        // solution as the average of the last two, as per Stefcal. This speeds
        // up convergence.
        if iteration % 2 == 0 {
            // Update the DI Jones matrices, and for each pair of Jones matrices
            // in `old_jones` and `di_jones`, form a maximum "distance" between
            // elements of the Jones matrices.
            di_jones
                .outer_iter_mut()
                .zip(old_jones.outer_iter())
                .zip(precisions.outer_iter_mut())
                .zip(failed.iter())
                .filter(|(_, &failed)| !failed)
                .for_each(|(((mut di_jones, old_jones), mut antenna_precision), _)| {
                    // antenna_precision = sum(norm_sqr(di_jones - old_jones)) / num_freqs

                    // Currently, this function only takes one frequency at a
                    // time. So there's no need to normalise by the number of
                    // frequencies. But, the following line should be used if
                    // there are multiple frequencies present.
                    // let num_freqs = di_jones.len_of(Axis(1));
                    let jones_diff_sum = di_jones.iter().zip(old_jones.iter()).fold(
                        [0.0, 0.0, 0.0, 0.0],
                        |acc, (&new, &old)| {
                            let diff = new - old;
                            [
                                acc[0] + diff[0].norm_sqr(),
                                acc[1] + diff[1].norm_sqr(),
                                acc[2] + diff[2].norm_sqr(),
                                acc[3] + diff[3].norm_sqr(),
                            ]
                        },
                    );

                    antenna_precision
                        .iter_mut()
                        .zip(jones_diff_sum)
                        .for_each(|(a, d)| {
                            // As above, the following line should be used if
                            // multiple frequencies are present.
                            // *a = d / num_freqs;
                            *a = d;
                        });

                    // di_jones = 0.5 * (di_jones + old_jones)
                    di_jones.iter_mut().zip(old_jones.iter().copied()).for_each(
                        |(di_jones, old_jones)| {
                            *di_jones = (*di_jones + old_jones) * 0.5;
                        },
                    )
                });

            // Stop iterating if we have reached the stop threshold.
            if precisions.iter().all(|&v| v < stop_threshold) {
                break;
            }
        }
        old_jones.assign(&di_jones);
    }

    // Set failed antennas to NaN.
    di_jones
        .outer_iter_mut()
        .zip(failed.iter())
        .filter(|(_, &failed)| failed)
        .for_each(|(mut di_jones, _)| {
            di_jones.fill(Jones::nan());
        });

    // max_precision = maximum(distances)
    let max_precision: f64 = precisions
        .as_slice()
        .unwrap()
        .chunks_exact(4)
        .zip(failed.iter())
        .filter(|(_, &failed)| !failed)
        .fold(f64::MIN, |acc, (antenna_precision, _)| {
            // Rust really doesn't want you to use the .max() iterator method on
            // floats...
            acc.max(antenna_precision[3])
                .max(antenna_precision[2])
                .max(antenna_precision[1])
                .max(antenna_precision[0])
        });

    let num_failed = failed.iter().filter(|&&f| f).count();
    let converged = {
        // If only 4 or fewer antennas remain, or we never reached the minimum
        // threshold level, mark the solution as failed.
        if num_tiles - num_failed <= 4 || max_precision > min_threshold {
            di_jones.fill(Jones::nan());
            false
        }
        // If converged within the minimum threshold, signal success.
        else {
            true
        }
    };

    CalibrationResult {
        num_iterations: iteration,
        converged,
        max_precision,
        num_failed,
        chanblock: None,
        i_chanblock: None,
    }
}

/// The following is the same as "MitchCal", equation 11 of Mitchell et al.
/// <https://ui.adsabs.harvard.edu/abs/2008ISTSP...2..707M/abstract>
///
/// The next iteration of gains is determined by summing the numerator ("top")
/// and denominator ("bot") of each antenna separately.
fn calibration_loop(
    data_tfb: ArrayView3<Jones<f32>>,
    model_tfb: ArrayView3<Jones<f32>>,
    di_jones: ArrayView1<Jones<f64>>,
    mut top: ArrayViewMut1<Jones<f64>>,
    mut bot: ArrayViewMut1<Jones<f64>>,
) {
    let num_tiles = di_jones.len_of(Axis(0));

    // Time axis.
    data_tfb
        .outer_iter()
        .zip(model_tfb.outer_iter())
        .for_each(|(data_fb, model_fb)| {
            // Unflagged frequency chan axis.
            data_fb
                .outer_iter()
                .zip(model_fb.outer_iter())
                .for_each(|(data_b, model_b)| {
                    let mut i_tile1 = 0;
                    let mut i_tile2 = 0;

                    // Unflagged baseline axis.
                    #[allow(non_snake_case)]
                    data_b.iter().zip(model_b.iter()).for_each(|(data, model)| {
                        i_tile2 += 1;
                        if i_tile2 == num_tiles {
                            i_tile1 += 1;
                            i_tile2 = i_tile1 + 1;
                        }

                        let D = Jones::<f64>::from(data);
                        let M = Jones::<f64>::from(model);

                        // Suppress boundary checks for maximum performance!
                        unsafe {
                            // Tile 1
                            {
                                let J2 = *di_jones.uget(i_tile2);
                                let top_tile1 = top.uget_mut(i_tile1);
                                let bot_tile1 = bot.uget_mut(i_tile1);

                                // For tile 1, ( D G M^H ) / ( (M G^H) (M G^H)^H )
                                // let Z = G M^H
                                let Z = J2 * M.h();
                                // D (G M^H) = D Z
                                *top_tile1 += D * Z;
                                // (M G^H) (M G^H)^H = (G M^H)^H (G M^H) = Z^H Z
                                *bot_tile1 += Z.h() * Z;
                            }
                            // Tile 2
                            {
                                let J1 = *di_jones.uget(i_tile1);
                                let top_tile2 = top.uget_mut(i_tile2);
                                let bot_tile2 = bot.uget_mut(i_tile2);

                                // For tile 2, ( D^H G M ) / ( (G M)^H (G M) )
                                // let Z = G M
                                let Z = J1 * M;
                                // D^H G M = D^H Z
                                *top_tile2 += D.h() * Z;
                                // (G M)^H (G M) = Z^H Z
                                *bot_tile2 += Z.h() * Z;
                            }
                        }
                    });
                })
        });
}
