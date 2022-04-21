// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Generate sky-model visibilities from a sky-model source list.

mod error;
pub use error::VisSimulateError;

use std::collections::HashMap;
use std::ops::Deref;
use std::path::PathBuf;
use std::str::FromStr;

use clap::Parser;
use hifitime::Epoch;
use hifitime::{Duration, Unit};
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use itertools::{izip, Itertools};
use log::{debug, info};
use marlu::{
    precession::precess_time, Jones, LatLngHeight, MeasurementSetWriter,
    ObsContext as MarluObsContext, RADec, UvfitsWriter, VisContext, VisWritable, XyzGeodetic,
};
use mwalib::MetafitsContext;
use ndarray::prelude::*;

use crate::data_formats::VisOutputType;
use crate::{
    data_formats::{get_dipole_delays, get_dipole_gains},
    glob::get_single_match_from_glob,
    model,
};
use mwa_hyperdrive_beam::{create_fee_beam_object, create_no_beam_object, Beam, Delays};
use mwa_hyperdrive_common::{
    cfg_if, clap, hifitime, indicatif, itertools, log, marlu, mwalib, ndarray,
};
use mwa_hyperdrive_srclist::{
    read::read_source_list_file, veto_sources, ComponentCounts, SourceList,
    DEFAULT_CUTOFF_DISTANCE, DEFAULT_VETO_THRESHOLD, SOURCE_DIST_CUTOFF_HELP, VETO_THRESHOLD_HELP,
};

#[derive(Parser, Debug, Default)]
pub struct VisSimulateArgs {
    /// Path to the metafits file.
    #[clap(short, long, parse(from_str), help_heading = "INPUT AND OUTPUT")]
    metafits: PathBuf,

    /// Path to the output visibilities file.
    #[clap(
        short = 'o',
        long,
        default_value = "hyp_model.uvfits",
        help_heading = "INPUT AND OUTPUT"
    )]
    output_model_file: PathBuf,

    /// Path to the sky-model source list used for simulation.
    #[clap(short, long, help_heading = "INPUT AND OUTPUT")]
    source_list: String,

    /// The number of sources to use in the source list. The default is to use
    /// them all. Example: If 1000 sources are specified here, then the top 1000
    /// sources are used (based on their flux densities after the beam
    /// attenuation) within the specified source distance cutoff.
    #[clap(short, long, help_heading = "SKY-MODEL SOURCES")]
    num_sources: Option<usize>,

    #[clap(long, help = SOURCE_DIST_CUTOFF_HELP.as_str(), help_heading = "SKY-MODEL SOURCES")]
    source_dist_cutoff: Option<f64>,

    #[clap(long, help = VETO_THRESHOLD_HELP.as_str(), help_heading = "SKY-MODEL SOURCES")]
    veto_threshold: Option<f64>,

    /// Don't include sources containing point components in the input sky
    /// model.
    #[clap(long, help_heading = "SKY-MODEL SOURCES")]
    filter_points: bool,

    /// Don't include sources containing Gaussian components in the input sky
    /// model.
    #[clap(long, help_heading = "SKY-MODEL SOURCES")]
    filter_gaussians: bool,

    /// Don't include sources containing shapelet components in the input sky
    /// model.
    #[clap(long, help_heading = "SKY-MODEL SOURCES")]
    filter_shapelets: bool,

    /// The phase centre right ascension [degrees]. If this is not specified,
    /// then the metafits phase/pointing centre is used.
    #[clap(short, long, help_heading = "OBSERVATION PARAMETERS")]
    ra: Option<f64>,

    /// The phase centre declination [degrees]. If this is not specified, then
    /// the metafits phase/pointing centre is used.
    #[clap(short, long, help_heading = "OBSERVATION PARAMETERS")]
    dec: Option<f64>,

    /// The total number of fine channels in the observation.
    #[clap(
        short = 'c',
        long,
        default_value = "384",
        help_heading = "OBSERVATION PARAMETERS"
    )]
    num_fine_channels: usize,

    /// The fine-channel resolution [kHz].
    #[clap(
        short,
        long,
        default_value = "80",
        help_heading = "OBSERVATION PARAMETERS"
    )]
    freq_res: f64,

    /// The centroid frequency of the simulation [MHz]. If this is not
    /// specified, then the FREQCENT specified in the metafits is used.
    #[clap(long, help_heading = "OBSERVATION PARAMETERS")]
    middle_freq: Option<f64>,

    /// The number of time steps used from the metafits epoch.
    #[clap(
        short = 't',
        long,
        default_value = "14",
        help_heading = "OBSERVATION PARAMETERS"
    )]
    num_timesteps: usize,

    /// The time resolution [seconds].
    #[clap(long, default_value = "8", help_heading = "OBSERVATION PARAMETERS")]
    time_res: f64,

    /// Should we use a beam? Default is to use the FEE beam.
    #[clap(long, help_heading = "MODEL PARAMETERS")]
    no_beam: bool,

    /// The path to the HDF5 MWA FEE beam file. If not specified, this must be
    /// provided by the MWA_BEAM_FILE environment variable.
    #[clap(long, help_heading = "MODEL PARAMETERS")]
    beam_file: Option<PathBuf>,

    /// Pretend that all MWA dipoles are alive and well, ignoring whatever is in
    /// the metafits file.
    #[clap(long, help_heading = "MODEL PARAMETERS")]
    unity_dipole_gains: bool,

    /// Specify the MWA dipoles delays, ignoring whatever is in the metafits
    /// file.
    #[clap(long, multiple_values(true), help_heading = "MODEL PARAMETERS")]
    dipole_delays: Option<Vec<u32>>,

    /// Use the CPU for visibility generation. This is deliberately made
    /// non-default because using a GPU is much faster.
    #[cfg(feature = "cuda")]
    #[clap(long, help_heading = "MODEL PARAMETERS")]
    cpu: bool,

    /// When generating sky-model visibilities, don't draw progress bars.
    #[clap(long, help_heading = "USER INTERFACE")]
    no_progress_bars: bool,
}

impl VisSimulateArgs {
    pub fn run(&self, dry_run: bool) -> Result<(), VisSimulateError> {
        vis_simulate(self, dry_run)
    }
}

/// Parameters needed to do sky-model visibility simulation.
struct VisSimParams {
    /// Sky-model source list.
    source_list: SourceList,

    /// mwalib metafits context
    metafits: MetafitsContext,

    /// The output visibilities file.
    output_model_file: PathBuf,

    /// The phase centre.
    phase_centre: RADec,

    /// The fine frequency channel frequencies \[Hz\].
    fine_chan_freqs: Vec<f64>,

    freq_res_hz: f64,

    /// The [XyzGeodetic] positions of the tiles.
    xyzs: Vec<XyzGeodetic>,

    /// A map from baseline index to the baseline's constituent tiles.
    baseline_to_tile_map: HashMap<usize, (usize, usize)>,

    /// Flagged tiles.
    flagged_tiles: Vec<usize>,

    /// Timestamps to be simulated.
    timestamps: Vec<Epoch>,

    int_time: Duration,

    /// Interface to beam code.
    beam: Box<dyn Beam>,

    /// The Earth position of the interferometer.
    array_position: LatLngHeight,
}

impl VisSimParams {
    /// Convert arguments into parameters.
    fn new(args: &VisSimulateArgs) -> Result<VisSimParams, VisSimulateError> {
        debug!("{:#?}", &args);

        // Expose all the struct fields to ensure they're all used.
        let VisSimulateArgs {
            metafits,
            output_model_file,
            source_list,
            num_sources,
            source_dist_cutoff,
            veto_threshold,
            filter_points,
            filter_gaussians,
            filter_shapelets,
            ra,
            dec,
            num_fine_channels,
            freq_res,
            middle_freq,
            num_timesteps,
            time_res,
            no_beam,
            beam_file,
            unity_dipole_gains,
            dipole_delays,
            #[cfg(feature = "cuda")]
                cpu: _,
            no_progress_bars: _,
        } = args;

        // Read the metafits file with mwalib.
        // TODO: Allow the user to specify the mwa_version.
        let metafits = mwalib::MetafitsContext::new(&metafits, None)?;

        // Get the phase centre.
        let phase_centre = match (ra, dec, &metafits) {
            (Some(ra), Some(dec), _) => {
                // Verify that the input coordinates are sensible.
                if !(0.0..=360.0).contains(ra) {
                    return Err(VisSimulateError::RaInvalid);
                }
                if !(-90.0..=90.0).contains(dec) {
                    return Err(VisSimulateError::DecInvalid);
                }
                RADec::new_degrees(*ra, *dec)
            }
            (Some(_), None, _) => return Err(VisSimulateError::OnlyOneRAOrDec),
            (None, Some(_), _) => return Err(VisSimulateError::OnlyOneRAOrDec),
            (None, None, m) => {
                // The phase centre in a metafits file may not be present. If not,
                // we have to use the pointing centre.
                match (m.ra_phase_center_degrees, m.dec_phase_center_degrees) {
                    (Some(ra), Some(dec)) => RADec::new_degrees(ra, dec),
                    (None, None) => {
                        RADec::new_degrees(m.ra_tile_pointing_degrees, m.dec_tile_pointing_degrees)
                    }
                    _ => unreachable!(),
                }
            }
        };
        info!("Using phase centre {}", phase_centre);

        // Get the fine channel frequencies.
        if *num_fine_channels == 0 {
            return Err(VisSimulateError::FineChansZero);
        }
        if *freq_res < f64::EPSILON {
            return Err(VisSimulateError::FineChansWidthTooSmall);
        }
        info!("Number of fine channels: {}", num_fine_channels);
        info!("Fine-channel width:      {} kHz", freq_res);
        let middle_freq = middle_freq.unwrap_or(metafits.centre_freq_hz as _);
        let fine_chan_freqs = {
            let half_num_fine_chans = *num_fine_channels as f64 / 2.0;
            let freq_res = freq_res * 1000.0; // kHz -> Hz
            let mut fine_chan_freqs = Vec::with_capacity(*num_fine_channels);
            for i in 0..*num_fine_channels {
                fine_chan_freqs
                    .push(middle_freq - half_num_fine_chans * freq_res + freq_res * i as f64);
            }
            fine_chan_freqs
        };
        match fine_chan_freqs.as_slice() {
            [] => unreachable!(), // Handled above.
            [f] => info!("Only fine-channel freq: {} MHz", f / 1e6),
            [f_0, .., f_n] => {
                info!("First fine-channel freq: {} MHz", f_0 / 1e6);
                info!("Last fine-channel freq:  {} MHz", f_n / 1e6);
            }
        }

        // Populate the timestamps.
        let int_time = Duration::from_f64(*time_res, Unit::Second);
        let timestamps = {
            let mut timestamps = Vec::with_capacity(*num_timesteps);
            let start = Epoch::from_gpst_seconds(metafits.sched_start_gps_time_ms as f64 / 1e3);
            for i in 0..*num_timesteps {
                timestamps.push(start + int_time * i as i64);
            }
            timestamps
        };
        match timestamps.as_slice() {
            [] => return Err(VisSimulateError::ZeroTimeSteps),
            [t] => info!("Only timestep (GPS): {:.2}", t.as_gpst_seconds()),
            [t0, .., tn] => {
                info!("First timestep (GPS): {:.2}", t0.as_gpst_seconds());
                info!("Last timestep  (GPS): {:.2}", tn.as_gpst_seconds());
            }
        }

        // Get the geodetic XYZ coordinates of each of the MWA tiles.
        let xyzs = XyzGeodetic::get_tiles_mwa(&metafits);

        // Prepare a map between baselines and their constituent tiles.
        // TODO: Utilise tile flags.
        let flagged_tiles: Vec<usize> = vec![];
        let baseline_to_tile_map = {
            let mut baseline_to_tile_map = HashMap::new();
            let mut bl = 0;
            for tile1 in 0..metafits.num_ants {
                if flagged_tiles.contains(&tile1) {
                    continue;
                }
                for tile2 in tile1 + 1..metafits.num_ants {
                    if flagged_tiles.contains(&tile2) {
                        continue;
                    }
                    baseline_to_tile_map.insert(bl, (tile1, tile2));
                    bl += 1;
                }
            }
            baseline_to_tile_map
        };

        // Treat the specified source list as file path. Does it exist? Then use it.
        // Otherwise, treat the specified source list as a glob and attempt to find
        // a single file with it.
        let sl_pb = PathBuf::from(&source_list);
        let sl_pb = if sl_pb.exists() {
            sl_pb
        } else {
            get_single_match_from_glob(source_list)?
        };
        // Read the source list.
        // TODO: Allow the user to specify a source list type.
        let source_list = match read_source_list_file(&sl_pb, None) {
            Ok((sl, sl_type)) => {
                debug!("Successfully parsed {}-style source list", sl_type);
                sl
            }
            Err(e) => return Err(VisSimulateError::from(e)),
        };
        let ComponentCounts {
            num_points,
            num_gaussians,
            num_shapelets,
            ..
        } = source_list.get_counts();
        debug!("Found {num_points} points, {num_gaussians} gaussians, {num_shapelets} shapelets");

        // Apply any filters.
        let mut source_list = if *filter_points || *filter_gaussians || *filter_shapelets {
            let sl = source_list.filter(*filter_points, *filter_gaussians, *filter_shapelets);
            let ComponentCounts {
                num_points,
                num_gaussians,
                num_shapelets,
                ..
            } = sl.get_counts();
            debug!(
                "After filtering, there are {num_points} points, {num_gaussians} gaussians, {num_shapelets} shapelets"
            );
            sl
        } else {
            source_list
        };
        let beam = if *no_beam {
            create_no_beam_object(xyzs.len())
        } else {
            create_fee_beam_object(
                beam_file.as_ref(),
                metafits.num_ants,
                match dipole_delays {
                    Some(d) => {
                        if d.len() != 16 || d.iter().any(|&v| v > 32) {
                            return Err(VisSimulateError::BadDelays);
                        }
                        Delays::Partial(d.to_owned())
                    }
                    None => Delays::Full(get_dipole_delays(&metafits)),
                },
                match unity_dipole_gains {
                    true => None,
                    false => Some(get_dipole_gains(&metafits)),
                },
            )?
        };

        let array_position = LatLngHeight::new_mwa();
        let precession_info = precess_time(
            phase_centre,
            *timestamps.first().unwrap(),
            array_position.longitude_rad,
            array_position.latitude_rad,
        );

        // Get the coarse channel information out of the metafits file, but only
        // the ones aligned with the specified frequencies here.
        let coarse_chan_freqs: Vec<f64> = {
            let cc_width = f64::from(metafits.coarse_chan_width_hz);

            metafits
                .metafits_coarse_chans
                .iter()
                .map(|cc| f64::from(cc.chan_centre_hz))
                .filter(|cc_freq| {
                    fine_chan_freqs
                        .iter()
                        .any(|f| (*f as f64 - *cc_freq).abs() < cc_width / 2.0)
                })
                .collect()
        };

        veto_sources(
            &mut source_list,
            precession_info
                .hadec_j2000
                .to_radec(precession_info.lmst_j2000),
            precession_info.lmst_j2000,
            precession_info.array_latitude_j2000,
            &coarse_chan_freqs,
            beam.deref(),
            *num_sources,
            source_dist_cutoff.unwrap_or(DEFAULT_CUTOFF_DISTANCE),
            veto_threshold.unwrap_or(DEFAULT_VETO_THRESHOLD),
        )?;

        info!("Writing the sky model to {}", output_model_file.display());

        Ok(VisSimParams {
            source_list,
            metafits,
            output_model_file: output_model_file.to_owned(),
            phase_centre,
            fine_chan_freqs,
            freq_res_hz: freq_res * 1e3_f64,
            xyzs,
            baseline_to_tile_map,
            flagged_tiles,
            timestamps,
            int_time,
            beam,
            array_position,
        })
    }
}

/// Simulate sky-model visibilities from a sky-model source list.
fn vis_simulate(args: &VisSimulateArgs, dry_run: bool) -> Result<(), VisSimulateError> {
    // TODO: Display GPU info.
    cfg_if::cfg_if! {
        if #[cfg(feature = "cuda-single")] {
            if args.cpu {
                info!("Generating sky model visibilities on the CPU");
            } else {
                info!("Generating sky model visibilities on the GPU (single precision)");
            }
        } else if #[cfg(feature = "cuda")] {
            if args.cpu {
                info!("Generating sky model visibilities on the CPU");
            } else {
                info!("Generating sky model visibilities on the GPU (double precision)");
            }
        } else {
            info!("Generating sky model visibilities on the CPU");
        }
    }

    let params = VisSimParams::new(args)?;

    if dry_run {
        info!("Dry run -- exiting now.");
        return Ok(());
    }

    let vis_ctx = VisContext {
        num_sel_timesteps: params.timestamps.len(),
        start_timestamp: params.timestamps[0],
        int_time: params.int_time,
        num_sel_chans: params.fine_chan_freqs.len(),
        start_freq_hz: params.fine_chan_freqs[0] as f64,
        freq_resolution_hz: params.freq_res_hz,
        sel_baselines: params
            .baseline_to_tile_map
            .values()
            .cloned()
            .sorted()
            .collect(),
        avg_time: 1,
        avg_freq: 1,
        num_vis_pols: 4,
    };
    let out_shape = vis_ctx.sel_dims();
    // fix time axis to 1
    let out_shape = (1, out_shape.1, out_shape.2);

    // Construct our visibilities array. This will be re-used for each timestep
    // before it's written to disk. Simulated vis is [baseline][chan]
    let mut vis_model_timestep: Array2<Jones<f32>> =
        Array2::from_elem((out_shape.2, out_shape.1), Jones::default());
    debug!(
        "Shape of model array: ({} baselines, {} channels; {} MiB) (×2)",
        out_shape.2,
        out_shape.1,
        out_shape.2 * out_shape.1 * std::mem::size_of_val(&vis_model_timestep[[0, 0]])
        // 1024 * 1024 == 1 MiB.
        / 1024 / 1024
    );

    // vis output requires [timestep][chan][baseline], this is re-used.
    let mut vis_out: Array3<Jones<f32>> = Array3::from_elem(out_shape, Jones::default());
    let weight_out = Array3::from_elem(out_shape, vis_ctx.weight_factor() as f32);

    // Prepare the output visibilities file.

    let obs_name = Some(format!(
        "Simulated visibilities for obsid {}",
        params.metafits.obs_id
    ));

    let output_type = match params
        .output_model_file
        .extension()
        .and_then(|e| e.to_str())
    {
        Some(s) => {
            VisOutputType::from_str(s).map_err(|e| VisSimulateError::OutputFileExtension {
                path: format!("{}", &params.output_model_file.display()),
                message: e.to_string(),
            })?
        }
        None => {
            return Err(VisSimulateError::OutputFileExtension {
                path: format!("{}", &params.output_model_file.display()),
                message: "no extension".to_string(),
            })
        }
    };

    let tile_names: Vec<&str> = params
        .metafits
        .rf_inputs
        .iter()
        .filter(|rf| rf.pol == mwalib::Pol::X)
        .map(|rf| rf.tile_name.as_str())
        .collect();

    let mut output_writer: Box<dyn VisWritable> = match output_type {
        VisOutputType::Uvfits => {
            let writer = UvfitsWriter::from_marlu(
                &params.output_model_file,
                &vis_ctx,
                Some(params.array_position),
                params.phase_centre,
                obs_name,
            )?;
            Box::new(writer)
        }
        VisOutputType::MeasurementSet => {
            let writer = MeasurementSetWriter::new(
                &params.output_model_file,
                params.phase_centre,
                Some(params.array_position),
            );

            let sched_start_timestamp = vis_ctx.start_timestamp;

            let sched_duration =
                *params.timestamps.last().unwrap() + params.int_time - sched_start_timestamp;

            let marlu_obs_ctx = MarluObsContext {
                sched_start_timestamp,
                sched_duration,
                name: obs_name,
                phase_centre: params.phase_centre,
                // XXX(Dev): is this right?
                pointing_centre: None,
                array_pos: params.array_position,
                ant_positions_enh: params
                    .xyzs
                    .iter()
                    .map(|xyz| xyz.to_enh(params.array_position.latitude_rad))
                    .collect(),
                ant_names: tile_names.iter().map(|&s| s.to_string()).collect(),
                // TODO(dev): is there any value in adding this metadata via hyperdrive obs context?
                field_name: None,
                project_id: None,
                observer: None,
            };
            writer.initialize(&vis_ctx, &marlu_obs_ctx)?;
            Box::new(writer)
        }
    };

    // Create a "modeller" object.
    let modeller = model::new_sky_modeller(
        #[cfg(feature = "cuda")]
        args.cpu,
        params.beam.deref(),
        &params.source_list,
        &params.xyzs,
        &params.fine_chan_freqs,
        &params.flagged_tiles,
        params.phase_centre,
        params.array_position.longitude_rad,
        params.array_position.latitude_rad,
        // TODO: Allow the user to turn off precession.
        true,
    )?;

    // Progress bar.
    let model_progress = ProgressBar::new(params.timestamps.len() as _)
        .with_style(
            ProgressStyle::default_bar()
                .template(
                    "{msg}: [{wide_bar:.blue}] {pos:2}/{len:2} timesteps ({elapsed_precise}<{eta_precise})",
                )
                .progress_chars("=> "),
        )
        .with_position(0)
        .with_message("Sky modelling");
    model_progress.set_draw_target(if args.no_progress_bars {
        ProgressDrawTarget::hidden()
    } else {
        ProgressDrawTarget::stdout()
    });
    model_progress.tick();

    // Generate the visibilities.
    for &timestamp in params.timestamps.iter() {
        // Clear the visibilities before re-using the buffer.
        vis_model_timestep.fill(Jones::default());
        modeller.model_timestep(vis_model_timestep.view_mut(), timestamp)?;

        // transpose model vis to output ordering. first axis is baseline.
        for (vis_model, mut vis_out) in izip!(
            vis_model_timestep.outer_iter(),
            vis_out.axis_iter_mut(Axis(2))
        ) {
            // second axis is channel
            for (model_jones, mut vis_out) in
                izip!(vis_model.iter(), vis_out.axis_iter_mut(Axis(1)))
            {
                vis_out.fill(*model_jones);
            }
        }

        let chunk_vis_ctx = VisContext {
            start_timestamp: timestamp - params.int_time / 2.0,
            num_sel_timesteps: 1,
            ..vis_ctx.clone()
        };
        // Write the visibilities out.
        output_writer.write_vis_marlu(
            vis_out.view(),
            weight_out.view(),
            &chunk_vis_ctx,
            &params.xyzs,
            false,
        )?;

        model_progress.inc(1);
    }
    model_progress.finish_with_message("Finished generating sky model");

    // Finalise writing the model.
    if matches!(output_type, VisOutputType::Uvfits) {
        let uvfits_writer =
            unsafe { Box::from_raw(Box::into_raw(output_writer) as *mut UvfitsWriter) };
        uvfits_writer.write_uvfits_antenna_table(&tile_names, &params.xyzs)?;
    }
    info!(
        "Finished writing sky model to {}",
        &params.output_model_file.display()
    );

    Ok(())
}