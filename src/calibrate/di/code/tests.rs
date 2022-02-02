// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Direction-independent calibration tests.

use approx::assert_abs_diff_eq;
use marlu::{c64, time::gps_to_epoch, Jones, XyzGeodetic};
use ndarray::prelude::*;

use tempfile::tempdir;

use super::{calibrate, calibrate_timeblocks, get_cal_vis, CalVis, IncompleteSolutions};
use crate::{
    calibrate::{Chanblock, Timeblock},
    jones_test::TestJones,
    tests::reduced_obsids::get_reduced_1090008640,
};
use mwa_hyperdrive_common::{marlu, ndarray};

/// Make some data "four times as bright as the model". The solutions should
/// then be all "twos". As data and model visibilities are given per baseline
/// and solutions are given per tile, the per tile values should be the sqrt of
/// the multiplicative factor used.
#[test]
fn test_calibrate_trivial() {
    let num_timesteps = 1;
    let num_timeblocks = 1;
    let timeblock_length = 1;
    let num_tiles = 5;
    let num_baselines = num_tiles * (num_tiles - 1) / 2;
    let num_chanblocks = 1;

    let vis_shape = (num_timesteps, num_baselines, num_chanblocks);
    let vis_data: Array3<Jones<f32>> = Array3::from_elem(vis_shape, Jones::identity() * 4.0);
    let vis_model: Array3<Jones<f32>> = Array3::from_elem(vis_shape, Jones::identity());
    let mut di_jones = Array3::from_elem(
        (num_timeblocks, num_tiles, num_chanblocks),
        Jones::<f64>::identity(),
    );

    for timeblock in 0..num_timeblocks {
        let time_range_start = timeblock * timeblock_length;
        let time_range_end = ((timeblock + 1) * timeblock_length).min(vis_data.dim().0);

        let mut di_jones_rev = di_jones.slice_mut(s![timeblock, .., ..]).reversed_axes();

        for (chanblock_index, mut di_jones_rev) in (0..num_chanblocks)
            .into_iter()
            .zip(di_jones_rev.outer_iter_mut())
        {
            let range = s![
                time_range_start..time_range_end,
                ..,
                chanblock_index..chanblock_index + 1
            ];
            let vis_data_slice = vis_data.slice(range);
            let vis_model_slice = vis_model.slice(range);
            let result = calibrate(
                vis_data_slice,
                vis_model_slice,
                di_jones_rev.view_mut(),
                &vec![1.0; vis_data.dim().1],
                20,
                1e-8,
                1e-5,
            );

            assert!(result.converged);
            assert_eq!(result.num_iterations, 10);
            assert_eq!(result.num_failed, 0);
            assert!(result.max_precision < 1e-13);
            // The solutions should be 2 * identity.
            let expected = Array1::from_elem(di_jones_rev.len(), Jones::identity() * 2.0);

            let di_jones_rev = di_jones_rev.mapv(TestJones::from);
            let expected = expected.mapv(TestJones::from);
            assert_abs_diff_eq!(di_jones_rev, expected, epsilon = 1e-14);
        }
    }

    let di_jones = di_jones.mapv(TestJones::from);
    let expected = Array3::from_elem(di_jones.dim(), Jones::identity() * 2.0).mapv(TestJones::from);
    assert_abs_diff_eq!(di_jones, expected, epsilon = 1e-14);
}

/// Test that converting [IncompleteSolutions] to [CalibrationSolutions] does
/// what's expected.
#[test]
fn incomplete_to_complete() {
    let timeblocks = [Timeblock {
        index: 0,
        range: 0..1,
        start: gps_to_epoch(1065880128.0),
        end: gps_to_epoch(1065880130.0),
        average: gps_to_epoch(1065880129.0),
    }];
    let chanblocks = [
        Chanblock {
            chanblock_index: 0,
            unflagged_index: 0,
            freq: 150e6,
        },
        Chanblock {
            chanblock_index: 1,
            unflagged_index: 1,
            freq: 151e6,
        },
        Chanblock {
            chanblock_index: 2,
            unflagged_index: 2,
            freq: 152e6,
        },
    ];
    let flagged_tiles = [];
    let flagged_chanblock_indices = [];
    let num_timeblocks = timeblocks.len();
    let num_tiles = 5;
    let num_baselines = num_tiles * (num_tiles - 1) / 2;
    let num_chanblocks = chanblocks.len();
    let baseline_weights = vec![1.0; num_baselines];
    let incomplete = IncompleteSolutions {
        di_jones: Array3::from_elem(
            (num_timeblocks, num_tiles, num_chanblocks),
            Jones::identity() * 2.0,
        ),
        timeblocks: &timeblocks,
        chanblocks: &chanblocks,
        baseline_weights: &baseline_weights,
        max_iterations: 50,
        stop_threshold: 1e-8,
        min_threshold: 1e-4,
    };

    let all_tile_positions = vec![
        XyzGeodetic {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        num_tiles + flagged_tiles.len()
    ];
    let complete = incomplete.into_cal_sols(
        &all_tile_positions,
        &flagged_tiles,
        &flagged_chanblock_indices,
        Some(1065880128),
    );

    // The "complete" solutions should have inverted Jones matrices.
    let expected = Array3::from_elem(
        (num_timeblocks, num_tiles, num_chanblocks),
        (Jones::identity() * 2.0).inv(),
    )
    .mapv(TestJones::from);
    assert_abs_diff_eq!(complete.di_jones.mapv(TestJones::from), expected);

    assert!(complete.flagged_tiles.is_empty());
    assert!(complete.flagged_chanblocks.is_empty());

    // Let's now add some flags.
    let timeblocks = [Timeblock {
        index: 0,
        range: 0..1,
        start: gps_to_epoch(1065880128.0),
        end: gps_to_epoch(1065880130.0),
        average: gps_to_epoch(1065880129.0),
    }];
    let chanblocks = [
        Chanblock {
            chanblock_index: 0,
            unflagged_index: 0,
            freq: 150e6,
        },
        Chanblock {
            chanblock_index: 2,
            unflagged_index: 1,
            freq: 152e6,
        },
        Chanblock {
            chanblock_index: 3,
            unflagged_index: 2,
            freq: 153e6,
        },
    ];
    let flagged_tiles = [2];
    let flagged_chanblock_indices = [1];
    let num_timeblocks = timeblocks.len();
    let num_tiles = 5;
    let num_baselines = num_tiles * (num_tiles - 1) / 2;
    let num_chanblocks = chanblocks.len();
    let baseline_weights = vec![1.0; num_baselines];
    // The solutions are now dependent on unflagged tile index.
    let mut di_jones = Array3::from_elem(
        (num_timeblocks, num_tiles, num_chanblocks),
        Jones::identity(),
    );
    let mut i_unflagged_tile: usize = 0;
    for i_tile in 0..num_tiles + flagged_tiles.len() {
        if !flagged_tiles.contains(&i_tile) {
            di_jones
                .slice_mut(s![.., i_unflagged_tile, ..])
                .map_inplace(|v| *v *= (i_unflagged_tile + 1) as f64);
            i_unflagged_tile += 1;
        }
    }
    let incomplete = IncompleteSolutions {
        di_jones,
        timeblocks: &timeblocks,
        chanblocks: &chanblocks,
        baseline_weights: &baseline_weights,
        max_iterations: 50,
        stop_threshold: 1e-8,
        min_threshold: 1e-4,
    };

    let all_tile_positions = vec![
        XyzGeodetic {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        num_tiles + flagged_tiles.len()
    ];
    let complete = incomplete.into_cal_sols(
        &all_tile_positions,
        &flagged_tiles,
        &flagged_chanblock_indices,
        Some(1065880128),
    );

    let mut i_unflagged_tile = 0;
    for i_tile in 0..num_tiles + flagged_tiles.len() {
        let slice = complete.di_jones.slice(s![.., i_tile, ..]);
        if flagged_tiles.contains(&i_tile) {
            // Jones matrices filled with NaN cannot be compared against each
            // other. Convert these to matrices with "random" numbers.
            assert_abs_diff_eq!(
                slice
                    .mapv(|_| Jones::identity() * 543.0)
                    .mapv(TestJones::from),
                Array2::from_elem(slice.dim(), Jones::identity() * 543.0).mapv(TestJones::from)
            );
        } else {
            // Generate the expected results based on the unflagged tile index.
            let mut expected = Array2::from_elem(
                slice.dim(),
                (Jones::identity() * (i_unflagged_tile + 1) as f64).inv(),
            );
            for i_chan in 0..num_chanblocks + flagged_chanblock_indices.len() {
                if flagged_chanblock_indices.contains(&i_chan) {
                    expected
                        .slice_mut(s![.., i_chan])
                        .map_inplace(|v| *v = Jones::from([c64::new(f64::NAN, f64::NAN); 4]));
                }
            }

            // Remove NaNs and compare.
            let result = slice
                .mapv(|v| {
                    if v.any_nan() {
                        Jones::identity() * 456.0
                    } else {
                        v
                    }
                })
                .mapv(TestJones::from);
            let expected = expected
                .mapv(|v| {
                    if v.any_nan() {
                        Jones::identity() * 456.0
                    } else {
                        v
                    }
                })
                .mapv(TestJones::from);
            assert_abs_diff_eq!(result, expected);
            i_unflagged_tile += 1;
        }
    }

    assert_eq!(complete.flagged_tiles.len(), 1);
    assert!(complete.flagged_tiles.contains(&2));
    assert_eq!(complete.flagged_chanblocks.len(), 1);
    assert!(complete.flagged_chanblocks.contains(&1));
}

/// Make a toml argument file without a metafits file.
#[test]
fn test_1090008640_quality() {
    let mut args = get_reduced_1090008640(true);
    let temp_dir = tempdir().expect("Couldn't make temp dir");
    args.outputs = Some(vec![temp_dir.path().join("hyp_sols.fits")]);

    let result = args.into_params();
    let params = match result {
        Ok(r) => r,
        Err(e) => panic!("{}", e),
    };

    let CalVis {
        vis_data,
        vis_weights: _,
        vis_model,
    } = get_cal_vis(&params, false).expect("Couldn't read data and generate a model");

    let (_, cal_results) = calibrate_timeblocks(
        vis_data.view(),
        vis_model.view(),
        &params.timeblocks,
        &params.chanblocks,
        &params.baseline_weights,
        params.max_iterations,
        params.stop_threshold,
        params.min_threshold,
        false,
    );

    // Only one timeblock.
    assert_eq!(cal_results.dim().0, 1);

    // One chanblock needs 28 iterations and 2 need 24 iterations. The rest need
    // at most 22.
    let mut count_28 = 1;
    let mut count_24 = 3;
    for cal_result in cal_results {
        match cal_result.num_iterations {
            28 => {
                count_28 -= 1;
                assert_eq!(cal_result.chanblock, Some(2));
            }
            24 => {
                count_24 -= 1;
                assert!([7, 21, 28].contains(&cal_result.chanblock.unwrap()));
            }
            0 => panic!("0 iterations? Something is wrong."),
            _ => {
                if cal_result.num_iterations % 2 == 1 {
                    panic!("An odd number of iterations shouldn't be possible; at the time of writing, only even numbers are allowed.");
                } else if cal_result.num_iterations > 22 {
                    panic!("Too many iterations required: {:?}", cal_result)
                }
            }
        }

        assert!(cal_result.converged);
        assert_eq!(cal_result.num_failed, 0);
        assert!(cal_result.max_precision < 1e8);
    }
    assert_eq!(count_28, 0);
    assert_eq!(count_24, 0);
}
