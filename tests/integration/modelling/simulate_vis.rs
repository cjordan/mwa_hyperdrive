// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Integration tests for sky-model visibilities generated by the "simulate-vis"
//! subcommand of hyperdrive.

use approx::assert_abs_diff_eq;
use clap::Parser;
use marlu::{XyzGeodetic, ENH};
use mwalib::{fitsio_sys, *};
use serial_test::serial;

use crate::*;
use mwa_hyperdrive::simulate_vis::{simulate_vis, SimulateVisArgs};
use mwa_hyperdrive_common::{cfg_if, clap, marlu, mwalib};

fn read_uvfits_stabxyz(
    fptr: &mut fitsio::FitsFile,
    _hdu: &fitsio::hdu::FitsHdu,
    num_tiles: usize,
) -> Vec<XyzGeodetic> {
    unsafe {
        // With the column name, get the column number.
        let mut status = 0;
        let mut col_num = -1;
        let keyword = std::ffi::CString::new("STABXYZ").unwrap();
        fitsio_sys::ffgcno(
            fptr.as_raw(),
            0,
            keyword.as_ptr() as *mut std::os::raw::c_char,
            &mut col_num,
            &mut status,
        );
        assert_eq!(status, 0, "Status wasn't 0");

        // Now get the column data.
        let mut array = vec![XyzGeodetic::default(); num_tiles];
        let array_ptr = array.as_mut_ptr();
        fitsio_sys::ffgcv(
            fptr.as_raw(),
            82, // TDOUBLE
            col_num,
            1,
            1,
            (num_tiles * 3) as i64,
            std::ptr::null_mut(),
            array_ptr as *mut core::ffi::c_void,
            &mut 0,
            &mut status,
        );
        assert_eq!(status, 0, "Status wasn't 0");
        array
    }
}

/// If di-calibrate is working, it should not write anything to stderr.
#[test]
fn test_no_stderr() {
    let num_timesteps = 2;
    let num_chans = 2;

    let mut output_path = TempDir::new().expect("couldn't make tmp dir").into_path();
    output_path.push("model.uvfits");
    let args = get_reduced_1090008640(true, false);
    let metafits = args.data.as_ref().unwrap()[0].clone();

    #[rustfmt::skip]
    let cmd = hyperdrive()
        .args(&[
            "simulate-vis",
            "--metafits", &metafits,
            "--source-list", &args.source_list.unwrap(),
            "--output-model-file", &format!("{}", output_path.display()),
            "--num-timesteps", &format!("{}", num_timesteps),
            "--num-fine-channels", &format!("{}", num_chans),
            "--no-progress-bars"
        ])
        .ok();
    assert!(cmd.is_ok(), "simulate-vis failed on simple test data!");
    let (_, stderr) = get_cmd_output(cmd);
    assert!(stderr.is_empty(), "stderr wasn't empty: {stderr}");
}

#[test]
#[serial]
fn test_1090008640_simulate_vis() {
    let num_timesteps = 2;
    let num_chans = 10;

    let mut output_path = TempDir::new().expect("couldn't make tmp dir").into_path();
    output_path.push("model.uvfits");
    let args = get_reduced_1090008640(true, false);
    let metafits = args.data.as_ref().unwrap()[0].clone();

    #[rustfmt::skip]
    let sim_args = SimulateVisArgs::parse_from(&[
        "simulate-vis",
        "--metafits", &metafits,
        "--source-list", &args.source_list.unwrap(),
        "--output-model-file", &format!("{}", output_path.display()),
        "--num-timesteps", &format!("{}", num_timesteps),
        "--num-fine-channels", &format!("{}", num_chans),
        "--no-progress-bars"
    ]);

    // Run simulate-vis and check that it succeeds
    let result = simulate_vis(
        sim_args,
        #[cfg(feature = "cuda")]
        false,
        false,
    );
    assert!(result.is_ok(), "result={:?} not ok", result.err().unwrap());

    // Test some metadata. Compare with the input metafits file.
    let metafits = MetafitsContext::new(&metafits, None).unwrap();
    let mut uvfits = fits_open!(&output_path).unwrap();
    let hdu = fits_open_hdu!(&mut uvfits, 0).unwrap();
    let gcount: String = get_required_fits_key!(&mut uvfits, &hdu, "GCOUNT").unwrap();
    let pcount: String = get_required_fits_key!(&mut uvfits, &hdu, "PCOUNT").unwrap();
    let floats_per_pol: String = get_required_fits_key!(&mut uvfits, &hdu, "NAXIS2").unwrap();
    let num_pols: String = get_required_fits_key!(&mut uvfits, &hdu, "NAXIS3").unwrap();
    let num_fine_freq_chans: String = get_required_fits_key!(&mut uvfits, &hdu, "NAXIS4").unwrap();
    let jd_zero: String = get_required_fits_key!(&mut uvfits, &hdu, "PZERO5").unwrap();
    let ptype4: String = get_required_fits_key!(&mut uvfits, &hdu, "PTYPE4").unwrap();

    assert_eq!(gcount.parse::<i32>().unwrap(), 16256);
    assert_eq!(pcount.parse::<i32>().unwrap(), 5);
    assert_eq!(floats_per_pol.parse::<i32>().unwrap(), 3);
    assert_eq!(num_pols.parse::<i32>().unwrap(), 4);
    assert_eq!(num_fine_freq_chans.parse::<i32>().unwrap(), 10);
    let jd_zero = jd_zero.parse::<f64>().unwrap();
    assert_abs_diff_eq!(jd_zero, 2.456860500E+06);
    assert_eq!(ptype4, "BASELINE");

    let hdu = fits_open_hdu!(&mut uvfits, 1).unwrap();
    let tile_names: Vec<String> = get_fits_col!(&mut uvfits, &hdu, "ANNAME").unwrap();
    assert_eq!(tile_names.len(), 128);
    assert_eq!(tile_names[0], "Tile011");
    assert_eq!(tile_names[1], "Tile012");
    assert_eq!(tile_names[127], "Tile168");
    for (i, (tile_name, metafits_tile_name)) in tile_names
        .iter()
        .zip(
            metafits
                .rf_inputs
                .iter()
                .filter(|rf| rf.pol == Pol::X)
                .map(|rf| &rf.tile_name),
        )
        .enumerate()
    {
        assert_eq!(tile_name, metafits_tile_name, "Wrong for tile {i}");
    }

    let tile_positions = read_uvfits_stabxyz(&mut uvfits, &hdu, 128);
    assert_abs_diff_eq!(tile_positions[0].x, 456.2500494643639);
    assert_abs_diff_eq!(tile_positions[0].y, -149.78500366210938);
    assert_abs_diff_eq!(tile_positions[0].z, 68.04598669887378);
    assert_abs_diff_eq!(tile_positions[10].x, 464.8409142556812);
    assert_abs_diff_eq!(tile_positions[10].y, -123.66699981689453);
    assert_abs_diff_eq!(tile_positions[10].z, 85.0377637878737);
    for (tile_pos, metafits_tile_pos) in
        tile_positions
            .into_iter()
            .zip(
                metafits
                    .rf_inputs
                    .iter()
                    .filter(|rf| rf.pol == Pol::X)
                    .map(|rf| {
                        ENH {
                            e: rf.east_m,
                            n: rf.north_m,
                            h: rf.height_m,
                        }
                        .to_xyz_mwa()
                    }),
            )
    {
        assert_abs_diff_eq!(tile_pos.x, metafits_tile_pos.x);
        assert_abs_diff_eq!(tile_pos.y, metafits_tile_pos.y);
        assert_abs_diff_eq!(tile_pos.z, metafits_tile_pos.z);
    }

    // Test visibility values.
    fits_open_hdu!(&mut uvfits, 0).unwrap();
    let mut group_params = [0.0; 5];
    let mut vis: Vec<f32> = vec![0.0; 10 * 4 * 3];
    let mut status = 0;
    unsafe {
        fitsio_sys::ffggpe(
            uvfits.as_raw(),           /* I - FITS file pointer                       */
            1,                         /* I - group to read (1 = 1st group)           */
            1,                         /* I - first vector element to read (1 = 1st)  */
            group_params.len() as i64, /* I - number of values to read                */
            group_params.as_mut_ptr(), /* O - array of values that are returned       */
            &mut status,               /* IO - error status                           */
        );
        assert_eq!(status, 0, "Status wasn't 0");
        fitsio_sys::ffgpve(
            uvfits.as_raw(),  /* I - FITS file pointer                       */
            1,                /* I - group to read (1 = 1st group)           */
            1,                /* I - first vector element to read (1 = 1st)  */
            vis.len() as i64, /* I - number of values to read                */
            0.0,              /* I - value for undefined pixels              */
            vis.as_mut_ptr(), /* O - array of values that are returned       */
            &mut 0,           /* O - set to 1 if any values are null; else 0 */
            &mut status,      /* IO - error status                           */
        );
        assert_eq!(status, 0, "Status wasn't 0");
    };

    assert_abs_diff_eq!(
        group_params[..],
        [
            -1.8128954e-7,
            -1.6615635e-8,
            -4.8240993e-9,
            258.0,
            -0.15944445
        ]
    );
    assert_abs_diff_eq!(group_params[4] as f64 + jd_zero, 2456860.3405555487);

    // The values of the visibilities changes slightly depending on the precision.
    cfg_if::cfg_if! {
        if #[cfg(feature = "cuda-single")] {
            assert_abs_diff_eq!(vis[0..29], [36.740772, -37.80606, 64.0, 36.464615, -38.027126, 64.0, 0.12835014, -0.07698195, 64.0, 0.13591047, -0.051941246, 64.0, 36.677788, -37.855072, 64.0, 36.411392, -38.083076, 64.0, 0.13199338, -0.07526354, 64.0, 0.13950738, -0.050253063, 64.0, 36.61131, -37.90193, 64.0, 36.354816, -38.13698]);
        } else {
            assert_abs_diff_eq!(vis[0..29], [36.740982, -37.80591, 64.0, 36.464863, -38.02699, 64.0, 0.12835437, -0.07698456, 64.0, 0.13591558, -0.05194349, 64.0, 36.677994, -37.85488, 64.0, 36.411633, -38.08291, 64.0, 0.13199718, -0.075266466, 64.0, 0.1395122, -0.050255615, 64.0, 36.61154, -37.901764, 64.0, 36.355083, -38.136826]);
        }
    }
    // Every third value (a weight) should be 64.
    for (i, vis) in vis.iter().enumerate() {
        if i % 3 == 2 {
            assert_abs_diff_eq!(*vis, 64.0);
        }
    }

    unsafe {
        fitsio_sys::ffggpe(
            uvfits.as_raw(),           /* I - FITS file pointer                       */
            8129,                      /* I - group to read (1 = 1st group)           */
            1,                         /* I - first vector element to read (1 = 1st)  */
            group_params.len() as i64, /* I - number of values to read                */
            group_params.as_mut_ptr(), /* O - array of values that are returned       */
            &mut status,               /* IO - error status                           */
        );
        assert_eq!(status, 0, "Status wasn't 0");
        fitsio_sys::ffgpve(
            uvfits.as_raw(),  /* I - FITS file pointer                       */
            8129,             /* I - group to read (1 = 1st group)           */
            1,                /* I - first vector element to read (1 = 1st)  */
            vis.len() as i64, /* I - number of values to read                */
            0.0,              /* I - value for undefined pixels              */
            vis.as_mut_ptr(), /* O - array of values that are returned       */
            &mut 0,           /* O - set to 1 if any values are null; else 0 */
            &mut status,      /* IO - error status                           */
        );
        assert_eq!(status, 0, "Status wasn't 0");
    };

    assert_abs_diff_eq!(
        group_params[..],
        [
            -1.8129641e-7,
            -1.6567755e-8,
            -4.729797e-9,
            258.0,
            -0.15935186
        ]
    );
    assert_abs_diff_eq!(group_params[4] as f64 + jd_zero, 2456860.3406481445);

    cfg_if::cfg_if! {
        if #[cfg(feature = "cuda-single")] {
            assert_abs_diff_eq!(vis[0..29], [36.799625, -37.71107, 64.0, 36.51971, -37.921703, 64.0, 0.12917347, -0.07720526, 64.0, 0.1368949, -0.052246865, 64.0, 36.734455, -37.760387, 64.0, 36.46407, -37.978012, 64.0, 0.13280584, -0.07548499, 64.0, 0.1404799, -0.050557088, 64.0, 36.665825, -37.807594, 64.0, 36.405113, -38.032326]);
        } else {
            assert_abs_diff_eq!(vis[0..29], [36.799675, -37.71089, 64.0, 36.519768, -37.921543, 64.0, 0.12917838, -0.07721107, 64.0, 0.13689607, -0.052250482, 64.0, 36.7345, -37.760174, 64.0, 36.464123, -37.97782, 64.0, 0.13281031, -0.07549093, 64.0, 0.14048097, -0.05056086, 64.0, 36.66584, -37.807365, 64.0, 36.405136, -38.032104]);
        }
    }
    for (i, vis) in vis.iter().enumerate() {
        if i % 3 == 2 {
            assert_abs_diff_eq!(*vis, 64.0);
        }
    }
}

// Ensure that visibilities generated by double-precision CUDA and the CPU are
// exactly the same.
#[test]
#[serial]
#[cfg(all(feature = "cuda", not(feature = "cuda-single")))]
fn test_1090008640_simulate_vis_cpu_gpu_match() {
    let num_timesteps = 2;
    let num_chans = 10;

    let mut output_path = TempDir::new().expect("couldn't make tmp dir").into_path();
    output_path.push("model.uvfits");
    let args = get_reduced_1090008640(true, false);
    let metafits = args.data.as_ref().unwrap()[0].clone();
    #[rustfmt::skip]
    let sim_args = SimulateVisArgs::parse_from(&[
        "simulate-vis",
        "--metafits", &metafits,
        "--source-list", &args.source_list.unwrap(),
        "--output-model-file", &format!("{}", output_path.display()),
        "--num-timesteps", &format!("{}", num_timesteps),
        "--num-fine-channels", &format!("{}", num_chans),
        "--no-progress-bars"
    ]);
    let result = simulate_vis(sim_args, true, false);
    assert!(result.is_ok(), "result={:?} not ok", result.err().unwrap());

    let mut uvfits = fits_open!(&output_path).unwrap();
    let hdu = fits_open_hdu!(&mut uvfits, 0).unwrap();

    let mut group_params = [0.0; 5];
    let mut vis_cpu: Vec<f32> = vec![0.0; 10 * 4 * 3];
    let mut status = 0;
    unsafe {
        fitsio_sys::ffggpe(
            uvfits.as_raw(),           /* I - FITS file pointer                       */
            1,                         /* I - group to read (1 = 1st group)           */
            1,                         /* I - first vector element to read (1 = 1st)  */
            group_params.len() as i64, /* I - number of values to read                */
            group_params.as_mut_ptr(), /* O - array of values that are returned       */
            &mut status,               /* IO - error status                           */
        );
        assert_eq!(status, 0, "Status wasn't 0");
        fitsio_sys::ffgpve(
            uvfits.as_raw(),      /* I - FITS file pointer                       */
            1,                    /* I - group to read (1 = 1st group)           */
            1,                    /* I - first vector element to read (1 = 1st)  */
            vis_cpu.len() as i64, /* I - number of values to read                */
            0.0,                  /* I - value for undefined pixels              */
            vis_cpu.as_mut_ptr(), /* O - array of values that are returned       */
            &mut 0,               /* O - set to 1 if any values are null; else 0 */
            &mut status,          /* IO - error status                           */
        );
        assert_eq!(status, 0, "Status wasn't 0");
    };
    drop(hdu);
    drop(uvfits);

    let args = get_reduced_1090008640(true, false);
    let metafits = args.data.as_ref().unwrap()[0].clone();
    #[rustfmt::skip]
    let sim_args = SimulateVisArgs::parse_from(&[
        "simulate-vis",
        "--metafits", &metafits,
        "--source-list", &args.source_list.unwrap(),
        "--output-model-file", &format!("{}", output_path.display()),
        "--num-timesteps", &format!("{}", num_timesteps),
        "--num-fine-channels", &format!("{}", num_chans),
        "--no-progress-bars"
    ]);

    // Run simulate-vis and check that it succeeds
    let result = simulate_vis(sim_args, false, false);
    assert!(result.is_ok(), "result={:?} not ok", result.err().unwrap());

    let mut uvfits = fits_open!(&output_path).unwrap();
    let hdu = fits_open_hdu!(&mut uvfits, 0).unwrap();

    let mut vis_gpu: Vec<f32> = vec![0.0; 10 * 4 * 3];
    unsafe {
        fitsio_sys::ffggpe(
            uvfits.as_raw(),           /* I - FITS file pointer                       */
            1,                         /* I - group to read (1 = 1st group)           */
            1,                         /* I - first vector element to read (1 = 1st)  */
            group_params.len() as i64, /* I - number of values to read                */
            group_params.as_mut_ptr(), /* O - array of values that are returned       */
            &mut status,               /* IO - error status                           */
        );
        assert_eq!(status, 0, "Status wasn't 0");
        fitsio_sys::ffgpve(
            uvfits.as_raw(),      /* I - FITS file pointer                       */
            1,                    /* I - group to read (1 = 1st group)           */
            1,                    /* I - first vector element to read (1 = 1st)  */
            vis_gpu.len() as i64, /* I - number of values to read                */
            0.0,                  /* I - value for undefined pixels              */
            vis_gpu.as_mut_ptr(), /* O - array of values that are returned       */
            &mut 0,               /* O - set to 1 if any values are null; else 0 */
            &mut status,          /* IO - error status                           */
        );
        assert_eq!(status, 0, "Status wasn't 0");
    };
    drop(hdu);
    drop(uvfits);

    for (cpu, gpu) in vis_cpu.into_iter().zip(vis_gpu) {
        assert_abs_diff_eq!(cpu, gpu)
    }
}
