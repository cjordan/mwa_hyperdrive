// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Code to read and write RTS calibration solutions.
//!
//! See for more info:
//! https://github.com/MWATelescope/mwa_hyperdrive/wiki/Calibration-solutions

// The RTS calls DI_JonesMatrices files "alignment files". The first two lines
// of these files correspond to "alignment flux density" (or "flux scale") and
// "post-alignment matrix". The reference line is structured the same as all
// those that follow it; 4 complex number pairs. All lines after the first two
// are "pre-alignment matrices". When reading in this data, the post-alignment
// matrix is inverted and applied to each pre-alignment matrix (as in, A*B where
// A is pre- and B- is inverse post).

mod read_files;

use read_files::*;

use std::path::{Path, PathBuf};

use crate::glob::get_all_matches_from_glob;
use log::trace;
use marlu::Jones;
use ndarray::prelude::*;
use regex::Regex;
use thiserror::Error;

use super::CalibrationSolutions;
use mwa_hyperdrive_common::{lazy_static, log, marlu, mwalib, ndarray, thiserror};

lazy_static::lazy_static! {
    static ref NODE_NUM: Regex = Regex::new(r"node(\d{3})\.dat$").unwrap();
}

pub(super) fn read<T: AsRef<Path>, T2: AsRef<Path>>(
    dir: T,
    metafits: T2,
) -> Result<CalibrationSolutions, RtsReadSolsError> {
    let context = mwalib::MetafitsContext::new(&metafits, None)?;
    for cc in &context.metafits_coarse_chans {
        println!(
            "{} {} {}",
            cc.corr_chan_number, cc.rec_chan_number, cc.gpubox_number
        );
    }

    // Search the current directory for DI_JonesMatrices_node???.dat and
    // BandpassCalibration_node???.dat files.
    let mut di_jm = get_all_matches_from_glob(&format!(
        "{}/DI_JonesMatrices_node???.dat",
        dir.as_ref().display()
    ))?;
    if di_jm.is_empty() {
        return Err(RtsReadSolsError::NoDiJmFiles {
            dir: dir.as_ref().display().to_string(),
        });
    }

    let mut bp_cal = get_all_matches_from_glob(&format!(
        "{}/BandpassCalibration_node???.dat",
        dir.as_ref().display()
    ))?;
    if bp_cal.is_empty() {
        return Err(RtsReadSolsError::NoBpCalFiles {
            dir: dir.as_ref().display().to_string(),
        });
    }

    trace!("Found RTS DI calibration files: {:?} {:?}", &di_jm, &bp_cal);

    // There should be an equal number of files for each type. The number of
    // files corresponds to the number of coarse channels.
    let num_coarse_chans = di_jm.len();
    if bp_cal.len() != num_coarse_chans {
        return Err(RtsReadSolsError::UnequalFileCount {
            dir: dir.as_ref().display().to_string(),
            di_jm_count: di_jm.len(),
            bp_cal_count: bp_cal.len(),
        });
    }

    // Sort the array of files by coarse channel.
    di_jm.sort_unstable_by(|a, b| {
        let a_node_num = &NODE_NUM.captures(&a.display().to_string()).unwrap()[1]
            .parse::<u8>()
            .unwrap();
        let b_node_num = &NODE_NUM.captures(&b.display().to_string()).unwrap()[1]
            .parse::<u8>()
            .unwrap();
        a_node_num.cmp(b_node_num)
    });
    bp_cal.sort_unstable_by(|a, b| {
        let a_node_num = &NODE_NUM.captures(&a.display().to_string()).unwrap()[1]
            .parse::<u8>()
            .unwrap();
        let b_node_num = &NODE_NUM.captures(&b.display().to_string()).unwrap()[1]
            .parse::<u8>()
            .unwrap();
        a_node_num.cmp(b_node_num)
    });

    // Unpack the files.
    let mut all_di_jm = vec![];
    for di_jm in di_jm {
        all_di_jm.push(
            DiJm::read_file(&di_jm).map_err(|err| RtsReadSolsError::DiJmError {
                filename: di_jm,
                err,
            })?,
        );
    }

    let mut all_bp_cal = vec![];
    for bp_cal in bp_cal {
        all_bp_cal.push(
            BpCal::read_file(&bp_cal).map_err(|err| RtsReadSolsError::BpCalError {
                filename: bp_cal,
                err,
            })?,
        );
    }

    // Check that the number of tiles is the same everywhere.
    let total_num_tiles = all_di_jm[0].pre_alignment_matrices.len();
    let num_unflagged_tiles = all_bp_cal[0].unflagged_tile_indices.len();
    for (node_index, di_jm) in all_di_jm.iter().enumerate() {
        if di_jm.pre_alignment_matrices.len() < num_unflagged_tiles {
            return Err(RtsReadSolsError::UnequalTileCountDiJm {
                expected: total_num_tiles,
                got: di_jm.pre_alignment_matrices.len(),
                node_index,
            });
        }
    }
    for (node_index, bp_cal) in all_bp_cal.iter().enumerate() {
        if bp_cal.data.dim().0 != num_unflagged_tiles {
            return Err(RtsReadSolsError::UnequalTileCountBpCal {
                expected: total_num_tiles,
                got: bp_cal.data.dim().0,
                node_index,
            });
        }
    }

    let num_unflagged_fine_chans = all_bp_cal
        .iter()
        .fold(0, |acc, bp_cal| acc + bp_cal.data.dim().2);
    // Try to work out the total number of fine frequency channels.
    let total_num_fine_freq_chans = {
        let smallest_fine_chan_res = all_bp_cal.iter().fold(f64::INFINITY, |acc, bp| {
            acc.min(bp.fine_channel_resolution.unwrap_or(f64::INFINITY))
        });

        // Only handling legacy correlator settings for now.
        if (smallest_fine_chan_res - 40e3).abs() < f64::EPSILON {
            768
        } else if (smallest_fine_chan_res - 20e3).abs() < f64::EPSILON {
            1536
        } else if (smallest_fine_chan_res - 10e3).abs() < f64::EPSILON {
            3072
        } else {
            panic!("Unhandled RTS frequency resolution")
        }
    };

    let mut di_jones = Array3::zeros((1, total_num_tiles, total_num_fine_freq_chans));
    let mut i_chan: usize = 0;
    for (mut bp_cal, di_jm) in all_bp_cal.into_iter().zip(all_di_jm.into_iter()) {
        let unflagged_tile_indices = bp_cal.unflagged_tile_indices;

        // Apply di_jm to the bp_cal data. Use the modified bp_cal data to
        // populate the outgoing di_jones.
        let num_chans = bp_cal.data.dim().2;
        let mut bp_cal = bp_cal.data.slice_mut(s![
            ..,
            0, // Throw away the "fit" data; only use "lsq".
            ..
        ]);
        bp_cal
            .outer_iter_mut()
            .zip(
                di_jm
                    .pre_alignment_matrices
                    .iter()
                    .enumerate()
                    .filter(|(i_tile, _)| unflagged_tile_indices.contains(i_tile))
                    .map(|pair| pair.1),
            )
            .for_each(|(mut bp_cal, &di_jm_pre)| {
                bp_cal.iter_mut().for_each(|bp_cal| {
                    *bp_cal = di_jm_pre * *bp_cal;

                    // These Jones matrices are currently in [PX, PY, QX, QY].
                    // Map them to [XX, XY, YX, YY].
                    *bp_cal = Jones::from([bp_cal[3], bp_cal[2], bp_cal[1], bp_cal[0]]);
                });
            });

        // Put unflagged data into the output di_jones. di_jones tiles are
        // ordered by metafits antenna number, whereas RTS data is ordered by
        // metafits input number. Flagged tiles have NaN written.
        let mut i_unflagged_tile: usize = 0;
        di_jones
            .slice_mut(s![0, .., i_chan..i_chan + num_chans])
            .outer_iter_mut()
            .enumerate()
            .for_each(|(i_ant, mut di_jones)| {
                let i_input = context
                    .rf_inputs
                    .iter()
                    .find(|rf| rf.ant as usize == i_ant)
                    .map(|rf| rf.input / 2)
                    .unwrap() as usize;
                if unflagged_tile_indices.contains(&i_input) {
                    di_jones.assign(&bp_cal.slice(s![i_unflagged_tile, ..]));
                    i_unflagged_tile += 1;
                } else {
                    di_jones.fill(Jones::nan());
                }
            });
        i_chan += num_chans
    }

    Ok(CalibrationSolutions {
        di_jones,
        // RTS solutions don't change over time.
        num_timeblocks: 1,
        total_num_tiles,
        unflagged_tiles: todo!(),
        total_num_fine_freq_chans,
        unflagged_fine_channels: todo!(),
        start_timestamps: vec![],
        obsid: Some(context.obs_id),
        time_res: None,
    })
}

#[derive(Error, Debug)]
pub enum RtsReadSolsError {
    #[error("Found no RTS DI_JonesMatrices_node???.dat files in directory {dir}")]
    NoDiJmFiles { dir: String },

    #[error("Found no RTS BandpassCalibration_node???.dat files in directory {dir}")]
    NoBpCalFiles { dir: String },

    #[error("In directory {dir}, found {di_jm_count} DI_JonesMatrices_node???.dat files, but {bp_cal_count} BandpassCalibration_node???.dat files.\nThere must be an equal number of these files.")]
    UnequalFileCount {
        dir: String,
        di_jm_count: usize,
        bp_cal_count: usize,
    },

    #[error("Expected {expected} tiles in all files, but got {got} tiles in DI_JonesMatrices_node{node_index:03}.dat")]
    UnequalTileCountDiJm {
        expected: usize,
        got: usize,
        node_index: usize,
    },

    #[error("Expected {expected} tiles in all files, but got {got} tiles in BandpassCalibration_node{node_index:03}.dat")]
    UnequalTileCountBpCal {
        expected: usize,
        got: usize,
        node_index: usize,
    },

    #[error("Error when reading {filename}: {0}")]
    DiJmError {
        filename: PathBuf,
        err: ReadDiJmFileError,
    },

    #[error("Error when reading {filename}: {0}")]
    BpCalError {
        filename: PathBuf,
        err: ReadBpCalFileError,
    },

    #[error("{0}")]
    Glob(#[from] crate::glob::GlobError),

    #[error("{0}")]
    Mwalib(#[from] mwalib::MwalibError),
}
