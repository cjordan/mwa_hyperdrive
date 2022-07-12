// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Integration tests for sky-model visibilities generated by the "vis-simulate"
//! subcommand of hyperdrive.

use approx::assert_abs_diff_eq;
use clap::Parser;
use marlu::{XyzGeodetic, ENH};
use mwalib::{fitsio_sys, *};
use serial_test::serial;
use tempfile::TempDir;

use crate::{tests::reduced_obsids::get_reduced_1090008640, vis_utils::simulate::VisSimulateArgs};
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
        // ffgcno = fits_get_colnum
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
        // ffgcv = fits_read_col
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

#[test]
#[serial]
fn test_1090008640_vis_simulate() {
    let num_timesteps = 2;
    let num_chans = 10;

    let temp_dir = TempDir::new().expect("couldn't make tmp dir");
    let output_path = temp_dir.path().join("model.uvfits");
    let args = get_reduced_1090008640(false);
    let metafits = args.data.as_ref().unwrap()[0].clone();

    #[rustfmt::skip]
    let sim_args = VisSimulateArgs::parse_from(&[
        "vis-simulate",
        "--metafits", &metafits,
        "--source-list", &args.source_list.unwrap(),
        "--output-model-files", &format!("{}", output_path.display()),
        "--num-timesteps", &format!("{}", num_timesteps),
        "--num-fine-channels", &format!("{}", num_chans),
        "--veto-threshold", "0.0", // Don't complicate things with vetoing
        "--no-progress-bars"
    ]);

    // Run vis-simulate and check that it succeeds
    let result = sim_args.run(false);
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
    let mut vis: Vec<f32> = vec![0.0; num_chans * 4 * 3];
    let mut status = 0;
    unsafe {
        // ffggpe = fits_read_grppar_flt
        fitsio_sys::ffggpe(
            uvfits.as_raw(),           /* I - FITS file pointer                       */
            1,                         /* I - group to read (1 = 1st group)           */
            1,                         /* I - first vector element to read (1 = 1st)  */
            group_params.len() as i64, /* I - number of values to read                */
            group_params.as_mut_ptr(), /* O - array of values that are returned       */
            &mut status,               /* IO - error status                           */
        );
        assert_eq!(status, 0, "Status wasn't 0");
        // ffgpve = fits_read_img_flt
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
            -1.812924e-7,
            -1.6595452e-8,
            -4.7857687e-9,
            258.0,
            -0.15939815
        ]
    );
    assert_abs_diff_eq!(group_params[4] as f64 + jd_zero, 2456860.3406018466);

    // The values of the visibilities changes slightly depending on the precision.
    cfg_if::cfg_if! {
        if #[cfg(feature = "cuda-single")] {
            assert_abs_diff_eq!(vis[0..29], [36.239098, -37.919975, 64.0, 36.509888, -37.688175, 64.0, 0.13714215, -0.051078193, 64.0, 0.12929335, -0.07608978, 64.0, 36.183933, -37.978363, 64.0, 36.445034, -37.739532, 64.0, 0.14075089, -0.049377773, 64.0, 0.13295034, -0.07435731, 64.0, 36.12551, -38.03485, 64.0, 36.376793, -37.788895]);
        } else {
            assert_abs_diff_eq!(vis[0..29], [36.23898, -37.920006, 64.0, 36.50975, -37.68825, 64.0, 0.13713828, -0.05107821, 64.0, 0.12928975, -0.07608952, 64.0, 36.183853, -37.97843, 64.0, 36.44494, -37.739647, 64.0, 0.14074738, -0.049377635, 64.0, 0.13294694, -0.07435694, 64.0, 36.125416, -38.034897, 64.0, 36.37668, -37.788986]);
        }
    }
    // Every third value (a weight) should be 64.
    for (i, vis) in vis.iter().enumerate() {
        if i % 3 == 2 {
            assert_abs_diff_eq!(*vis, 64.0);
        }
    }

    unsafe {
        // ffggpe = fits_read_grppar_flt
        fitsio_sys::ffggpe(
            uvfits.as_raw(),           /* I - FITS file pointer                       */
            8129,                      /* I - group to read (1 = 1st group)           */
            1,                         /* I - first vector element to read (1 = 1st)  */
            group_params.len() as i64, /* I - number of values to read                */
            group_params.as_mut_ptr(), /* O - array of values that are returned       */
            &mut status,               /* IO - error status                           */
        );
        assert_eq!(status, 0, "Status wasn't 0");
        // ffgpve = fits_read_img_flt
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
            -1.8129924e-7,
            -1.6547578e-8,
            -4.691462e-9,
            258.0,
            -0.15930556
        ]
    );
    assert_abs_diff_eq!(group_params[4] as f64 + jd_zero, 2456860.3406944424);

    cfg_if::cfg_if! {
        if #[cfg(feature = "cuda-single")] {
            assert_abs_diff_eq!(vis[0..29], [36.29235, -37.815643, 64.0, 36.566887, -37.594475, 64.0, 0.13810812, -0.051415868, 64.0, 0.1300995, -0.07634619, 64.0, 36.23478, -37.874428, 64.0, 36.499863, -37.646164, 64.0, 0.14170626, -0.049713243, 64.0, 0.13374655, -0.07461138, 64.0, 36.173973, -37.93127, 64.0, 36.42945, -37.69583]);
        } else {
            assert_abs_diff_eq!(vis[0..29], [36.29221, -37.81562, 64.0, 36.566708, -37.59445, 64.0, 0.13810773, -0.0514111, 64.0, 0.13010167, -0.07633954, 64.0, 36.234665, -37.87441, 64.0, 36.499706, -37.646156, 64.0, 0.14170499, -0.049708802, 64.0, 0.13374788, -0.07460498, 64.0, 36.173786, -37.931236, 64.0, 36.42923, -37.6958]);
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
fn test_1090008640_vis_simulate_cpu_gpu_match() {
    let num_timesteps = 2;
    let num_chans = 10;

    let temp_dir = TempDir::new().expect("couldn't make tmp dir");
    let output_path = temp_dir.path().join("model.uvfits");
    let args = get_reduced_1090008640(false);
    let metafits = args.data.as_ref().unwrap()[0].clone();
    #[rustfmt::skip]
    let sim_args = VisSimulateArgs::parse_from(&[
        "vis-simulate",
        "--metafits", &metafits,
        "--source-list", &args.source_list.unwrap(),
        "--output-model-files", &format!("{}", output_path.display()),
        "--num-timesteps", &format!("{}", num_timesteps),
        "--num-fine-channels", &format!("{}", num_chans),
        "--no-progress-bars",
        "--cpu",
    ]);
    let result = sim_args.run(false);
    assert!(result.is_ok(), "result={:?} not ok", result.err().unwrap());

    let mut uvfits = fits_open!(&output_path).unwrap();
    let hdu = fits_open_hdu!(&mut uvfits, 0).unwrap();

    let mut group_params = [0.0; 5];
    let mut vis_cpu: Vec<f32> = vec![0.0; num_chans * 4 * 3];
    let mut status = 0;
    unsafe {
        // ffggpe = fits_read_grppar_flt
        fitsio_sys::ffggpe(
            uvfits.as_raw(),           /* I - FITS file pointer                       */
            1,                         /* I - group to read (1 = 1st group)           */
            1,                         /* I - first vector element to read (1 = 1st)  */
            group_params.len() as i64, /* I - number of values to read                */
            group_params.as_mut_ptr(), /* O - array of values that are returned       */
            &mut status,               /* IO - error status                           */
        );
        assert_eq!(status, 0, "Status wasn't 0");
        // ffgpve = fits_read_img_flt
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

    let args = get_reduced_1090008640(false);
    let metafits = args.data.as_ref().unwrap()[0].clone();
    #[rustfmt::skip]
    let sim_args = VisSimulateArgs::parse_from(&[
        "vis-simulate",
        "--metafits", &metafits,
        "--source-list", &args.source_list.unwrap(),
        "--output-model-files", &format!("{}", output_path.display()),
        "--num-timesteps", &format!("{}", num_timesteps),
        "--num-fine-channels", &format!("{}", num_chans),
        "--no-progress-bars"
    ]);

    // Run vis-simulate and check that it succeeds
    let result = sim_args.run(false);
    assert!(result.is_ok(), "result={:?} not ok", result.err().unwrap());

    let mut uvfits = fits_open!(&output_path).unwrap();
    let hdu = fits_open_hdu!(&mut uvfits, 0).unwrap();

    let mut vis_gpu: Vec<f32> = vec![0.0; num_chans * 4 * 3];
    unsafe {
        // ffggpe = fits_read_grppar_flt
        fitsio_sys::ffggpe(
            uvfits.as_raw(),           /* I - FITS file pointer                       */
            1,                         /* I - group to read (1 = 1st group)           */
            1,                         /* I - first vector element to read (1 = 1st)  */
            group_params.len() as i64, /* I - number of values to read                */
            group_params.as_mut_ptr(), /* O - array of values that are returned       */
            &mut status,               /* IO - error status                           */
        );
        assert_eq!(status, 0, "Status wasn't 0");
        // ffgpve = fits_read_img_flt
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
