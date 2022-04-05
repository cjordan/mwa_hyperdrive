// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use approx::assert_abs_diff_eq;
use hifitime::Epoch;
use marlu::{c64, Jones};
use ndarray::prelude::*;

use super::*;
use crate::{jones_test::TestJones, tests::reduced_obsids::get_reduced_1090008640};
use mwa_hyperdrive_common::{hifitime, marlu, ndarray};

fn make_solutions() -> CalibrationSolutions {
    let num_timeblocks = 2;
    let num_tiles = 128;
    let num_chanblocks = 768;
    let di_jones = Array1::range(
        1.0,
        (num_timeblocks * num_tiles * num_chanblocks + 1) as f64,
        1.0,
    );
    let mut di_jones = di_jones
        .into_shape((num_timeblocks, num_tiles, num_chanblocks))
        .unwrap()
        .mapv(|v| {
            Jones::from([
                c64::new(1.01, 1.02),
                c64::new(1.03, 1.04),
                c64::new(1.05, 1.06),
                c64::new(1.07, 1.08),
            ]) * v
        });
    // Sprinkle some flags.
    let flagged_tiles = 3..5;
    let flagged_chanblocks = 5..8;
    di_jones
        .slice_mut(s![.., flagged_tiles.clone(), ..])
        .fill(Jones::nan());
    di_jones
        .slice_mut(s![.., .., flagged_chanblocks.clone()])
        .fill(Jones::nan());

    CalibrationSolutions {
        di_jones,
        flagged_tiles: flagged_tiles.into_iter().collect(),
        flagged_chanblocks: flagged_chanblocks.into_iter().map(|i| i as u16).collect(),
        obsid: Some(1090008640),
        start_timestamps: vec![
            Epoch::from_gpst_seconds(1090008640.0),
            Epoch::from_gpst_seconds(1090008650.0),
        ],
        end_timestamps: vec![
            Epoch::from_gpst_seconds(1090008650.0),
            Epoch::from_gpst_seconds(1090008660.0),
        ],
        average_timestamps: vec![
            Epoch::from_gpst_seconds(1090008645.0),
            Epoch::from_gpst_seconds(1090008655.0),
        ],
    }
}

#[test]
fn test_write_and_read_hyperdrive_solutions() {
    let sols = make_solutions();
    let tmp_file = tempfile::NamedTempFile::new().expect("Couldn't make tmp file");
    let result = hyperdrive::write(&sols, tmp_file.path());
    assert!(result.is_ok());
    result.unwrap();

    let result = hyperdrive::read(tmp_file.path());
    assert!(result.is_ok());
    let sols_from_disk = result.unwrap();

    assert_eq!(sols.di_jones.dim(), sols_from_disk.di_jones.dim());
    // Can't use assert_abs_diff_eq on the whole array, because it rejects NaN
    // equality.
    sols.di_jones
        .mapv(TestJones::from)
        .into_iter()
        .zip(sols_from_disk.di_jones.mapv(TestJones::from).into_iter())
        .for_each(|(expected, result)| {
            if expected.any_nan() {
                assert!(result.any_nan());
            } else {
                assert_abs_diff_eq!(expected, result);
            }
        });

    // TODO: Test the timestamps.
}

#[test]
fn test_write_and_read_ao_solutions() {
    let sols = make_solutions();
    let tmp_file = tempfile::NamedTempFile::new().expect("Couldn't make tmp file");
    let result = ao::write(&sols, tmp_file.path());
    assert!(result.is_ok());
    result.unwrap();

    let result = ao::read(tmp_file.path());
    assert!(result.is_ok());
    let sols_from_disk = result.unwrap();

    assert_eq!(sols.di_jones.dim(), sols_from_disk.di_jones.dim());
    // Can't use assert_abs_diff_eq on the whole array, because it rejects NaN
    // equality.
    sols.di_jones
        .mapv(TestJones::from)
        .into_iter()
        .zip(sols_from_disk.di_jones.mapv(TestJones::from).into_iter())
        .for_each(|(expected, result)| {
            if expected.any_nan() {
                assert!(result.any_nan());
            } else {
                assert_abs_diff_eq!(expected, result);
            }
        });

    // TODO: Test the timestamps.
}

#[test]
fn test_write_and_read_rts_solutions() {
    let sols = make_solutions();
    let args = get_reduced_1090008640(false);
    let metafits = &args.data.unwrap()[0];
    let tmp_dir = tempfile::tempdir().expect("Couldn't make tmp dir");

    let result = rts::write(&sols, tmp_dir.path(), metafits);
    assert!(result.is_ok(), "{}", result.unwrap_err());
    result.unwrap();

    let result = rts::read(tmp_dir.path(), metafits);
    let sols_from_disk = match result {
        Ok(r) => r,
        Err(e) => panic!("{}", e),
    };

    // `sols` has 2 timeblocks, but RTS solutions can only ever have 1.
    assert_eq!(sols_from_disk.di_jones.dim().0, 1);
    assert_eq!(sols.di_jones.dim().1, sols_from_disk.di_jones.dim().1);
    assert_eq!(sols.di_jones.dim().2, sols_from_disk.di_jones.dim().2);
    // Can't use assert_abs_diff_eq on the whole array, because it rejects NaN
    // equality.
    sols.di_jones
        .mapv(TestJones::from)
        .into_iter()
        .zip(sols_from_disk.di_jones.mapv(TestJones::from).into_iter())
        .for_each(|(expected, result)| {
            if expected.any_nan() {
                assert!(result.any_nan());
            } else {
                assert_abs_diff_eq!(
                    expected,
                    result,
                    // lmao
                    epsilon = 0.06
                );
            }
        });

    assert!(sols_from_disk.start_timestamps.is_empty());
    assert!(sols_from_disk.end_timestamps.is_empty());
    assert!(sols_from_disk.average_timestamps.is_empty());
}
