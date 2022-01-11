// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Types to generate sky-models with CUDA.

use std::collections::HashSet;

use marlu::{
    cuda_runtime_sys, pos::xyz::xyzs_to_cross_uvws_parallel, AzEl, Jones, RADec, XyzGeodetic, LMN,
    UVW,
};
use ndarray::prelude::*;
use rayon::prelude::*;

use crate as cuda;
use crate::{CudaFloat, CudaJones};
use mwa_hyperdrive_beam::{
    cuda_status_to_error, Beam, BeamCUDA, BeamError, DevicePointer,
    ERROR_STR_LENGTH as CUDA_ERROR_STR_LENGTH,
};
use mwa_hyperdrive_common::{cfg_if, marlu, ndarray, rayon};
use mwa_hyperdrive_srclist::{
    get_instrumental_flux_densities, ComponentType, FluxDensityType, ShapeletCoeff, SourceList,
};

/// The first axis of `*_list_fds` is unflagged fine channel frequency, the
/// second is the source component. The length of `hadecs`, `lmns`,
/// `*_list_fds`'s second axis are the same.
// TODO: Curved power laws.
pub struct SkyModellerCuda<'a> {
    cuda_beam: Box<dyn BeamCUDA>,

    /// The latitude of the array we're using.
    array_latitude_rad: f64,

    freqs: Vec<CudaFloat>,

    /// The [XyzGeodetic] positions of each of the unflagged tiles.
    unflagged_tile_xyzs: &'a [XyzGeodetic],

    /// A simple map from an absolute tile index into an unflagged tile index.
    /// This is important because CUDA will use tile indices from 0 to the
    /// length of `unflagged_tile_xyzs`, but the beam code has dipole delays and
    /// dipole gains available for *all* tiles. So if tile 32 is flagged, any
    /// CUDA thread with a tile index of 32 would naively get the flagged beam
    /// info. This map would make tile index go to the next unflagged tile,
    /// perhaps 33.
    tile_index_to_unflagged_tile_index_map: DevicePointer<i32>,

    sbf_l: i32,
    sbf_n: i32,
    sbf_c: CudaFloat,
    sbf_dx: CudaFloat,

    d_vis: DevicePointer<f32>,
    d_freqs: DevicePointer<CudaFloat>,
    d_shapelet_basis_values: DevicePointer<CudaFloat>,

    point_power_law_radecs: Vec<RADec>,
    point_power_law_lmns: DevicePointer<cuda::LMN>,
    /// Instrumental flux densities calculated at 150 MHz.
    point_power_law_fds: DevicePointer<CudaJones>,
    /// Spectral indices.
    point_power_law_sis: DevicePointer<CudaFloat>,

    point_list_radecs: Vec<RADec>,
    point_list_lmns: DevicePointer<cuda::LMN>,
    /// Instrumental (i.e. XX, XY, YX, XX).
    point_list_fds: DevicePointer<CudaJones>,

    gaussian_power_law_radecs: Vec<RADec>,
    gaussian_power_law_lmns: DevicePointer<cuda::LMN>,
    /// Instrumental flux densities calculated at 150 MHz.
    gaussian_power_law_fds: DevicePointer<CudaJones>,
    /// Spectral indices.
    gaussian_power_law_sis: DevicePointer<CudaFloat>,
    gaussian_power_law_gps: DevicePointer<cuda::GaussianParams>,

    gaussian_list_radecs: Vec<RADec>,
    gaussian_list_lmns: DevicePointer<cuda::LMN>,
    /// Instrumental (i.e. XX, XY, YX, XX).
    gaussian_list_fds: DevicePointer<CudaJones>,
    gaussian_list_gps: DevicePointer<cuda::GaussianParams>,

    shapelet_power_law_radecs: Vec<RADec>,
    shapelet_power_law_lmns: DevicePointer<cuda::LMN>,
    /// Instrumental flux densities calculated at 150 MHz.
    shapelet_power_law_fds: DevicePointer<CudaJones>,
    /// Spectral indices.
    shapelet_power_law_sis: DevicePointer<CudaFloat>,
    shapelet_power_law_gps: DevicePointer<cuda::GaussianParams>,
    shapelet_power_law_coeffs: DevicePointer<cuda::ShapeletCoeff>,
    shapelet_power_law_coeff_lens: DevicePointer<usize>,

    shapelet_list_radecs: Vec<RADec>,
    shapelet_list_lmns: DevicePointer<cuda::LMN>,
    /// Instrumental (i.e. XX, XY, YX, XX).
    shapelet_list_fds: DevicePointer<CudaJones>,
    shapelet_list_gps: DevicePointer<cuda::GaussianParams>,
    shapelet_list_coeffs: DevicePointer<cuda::ShapeletCoeff>,
    shapelet_list_coeff_lens: DevicePointer<usize>,
}

impl<'a> SkyModellerCuda<'a> {
    /// Given a source list, split the components into each component type (e.g.
    /// points, shapelets) and by each flux density type (e.g. list, power law),
    /// then copy them to a GPU ready for modelling. Where possible, list flux
    /// density types should be converted to power laws before calling this
    /// function, because using power laws is more efficient and probably more
    /// accurate.
    ///
    /// # Safety
    ///
    /// This function interfaces directly with the CUDA API. Rust errors attempt
    /// to catch problems but there are no guarantees.
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn new(
        beam: &dyn Beam,
        source_list: &SourceList,
        unflagged_fine_chan_freqs: &[f64],
        unflagged_tile_xyzs: &'a [XyzGeodetic],
        tile_flags: &HashSet<usize>,
        phase_centre: RADec,
        array_latitude_rad: f64,
        shapelet_basis_values: &[f64],
        sbf_l: usize,
        sbf_n: usize,
        sbf_c: f64,
        sbf_dx: f64,
    ) -> Result<SkyModellerCuda<'a>, BeamError> {
        let mut point_power_law_radecs: Vec<RADec> = vec![];
        let mut point_power_law_lmns: Vec<cuda::LMN> = vec![];
        let mut point_power_law_fds: Vec<_> = vec![];
        let mut point_power_law_sis: Vec<_> = vec![];

        let mut point_list_radecs: Vec<RADec> = vec![];
        let mut point_list_lmns: Vec<cuda::LMN> = vec![];
        let mut point_list_fds: Vec<FluxDensityType> = vec![];

        let mut gaussian_power_law_radecs: Vec<RADec> = vec![];
        let mut gaussian_power_law_lmns: Vec<cuda::LMN> = vec![];
        let mut gaussian_power_law_fds: Vec<_> = vec![];
        let mut gaussian_power_law_sis: Vec<_> = vec![];
        let mut gaussian_power_law_gps: Vec<cuda::GaussianParams> = vec![];

        let mut gaussian_list_radecs: Vec<RADec> = vec![];
        let mut gaussian_list_lmns: Vec<cuda::LMN> = vec![];
        let mut gaussian_list_fds: Vec<FluxDensityType> = vec![];
        let mut gaussian_list_gps: Vec<cuda::GaussianParams> = vec![];

        let mut shapelet_power_law_radecs: Vec<RADec> = vec![];
        let mut shapelet_power_law_lmns: Vec<cuda::LMN> = vec![];
        let mut shapelet_power_law_fds: Vec<_> = vec![];
        let mut shapelet_power_law_sis: Vec<_> = vec![];
        let mut shapelet_power_law_gps: Vec<cuda::GaussianParams> = vec![];
        let mut shapelet_power_law_coeffs: Vec<Vec<ShapeletCoeff>> = vec![];

        let mut shapelet_list_radecs: Vec<RADec> = vec![];
        let mut shapelet_list_lmns: Vec<cuda::LMN> = vec![];
        let mut shapelet_list_fds: Vec<FluxDensityType> = vec![];
        let mut shapelet_list_gps: Vec<cuda::GaussianParams> = vec![];
        let mut shapelet_list_coeffs: Vec<Vec<ShapeletCoeff>> = vec![];

        cfg_if::cfg_if! {
            if #[cfg(feature = "cuda-single")] {
                let jones_to_cuda_jones = |j: Jones<f64>| -> cuda::JonesF32 {
                    cuda::JonesF32 {
                        xx_re: j[0].re as f32,
                        xx_im: j[0].im as f32,
                        xy_re: j[1].re as f32,
                        xy_im: j[1].im as f32,
                        yx_re: j[2].re as f32,
                        yx_im: j[2].im as f32,
                        yy_re: j[3].re as f32,
                        yy_im: j[3].im as f32,
                    }
                };
            } else {
                let jones_to_cuda_jones = |j: Jones<f64>| -> cuda::JonesF64 {
                    cuda::JonesF64 {
                        xx_re: j[0].re,
                        xx_im: j[0].im,
                        xy_re: j[1].re,
                        xy_im: j[1].im,
                        yx_re: j[2].re,
                        yx_im: j[2].im,
                        yy_re: j[3].re,
                        yy_im: j[3].im,
                    }
                };
            }
        }

        for comp in source_list.iter().flat_map(|(_, src)| &src.components) {
            let radec = comp.radec;
            let LMN { l, m, n } = comp.radec.to_lmn(phase_centre).prepare_for_rime();
            let lmn = cuda::LMN {
                l: l as CudaFloat,
                m: m as CudaFloat,
                n: n as CudaFloat,
            };
            match &comp.comp_type {
                ComponentType::Point => match comp.flux_type {
                    FluxDensityType::PowerLaw { si, .. } => {
                        point_power_law_radecs.push(radec);
                        point_power_law_lmns.push(lmn);
                        let fd_at_150mhz = comp.estimate_at_freq(cuda::POWER_LAW_FD_REF_FREQ as _);
                        let inst_fd: Jones<f64> = fd_at_150mhz.to_inst_stokes();
                        let cuda_inst_fd = jones_to_cuda_jones(inst_fd);
                        point_power_law_fds.push(cuda_inst_fd);
                        point_power_law_sis.push(si as CudaFloat);
                    }

                    FluxDensityType::CurvedPowerLaw { .. } => todo!(),

                    FluxDensityType::List { .. } => {
                        point_list_radecs.push(radec);
                        point_list_lmns.push(lmn);
                        point_list_fds.push(comp.flux_type.clone());
                    }
                },

                ComponentType::Gaussian { maj, min, pa } => {
                    let gp = cuda::GaussianParams {
                        maj: *maj as CudaFloat,
                        min: *min as CudaFloat,
                        pa: *pa as CudaFloat,
                    };
                    match comp.flux_type {
                        FluxDensityType::PowerLaw { si, .. } => {
                            gaussian_power_law_radecs.push(radec);
                            gaussian_power_law_lmns.push(lmn);
                            let fd_at_150mhz =
                                comp.estimate_at_freq(cuda::POWER_LAW_FD_REF_FREQ as _);
                            let inst_fd: Jones<f64> = fd_at_150mhz.to_inst_stokes();
                            let cuda_inst_fd = jones_to_cuda_jones(inst_fd);
                            gaussian_power_law_fds.push(cuda_inst_fd);
                            gaussian_power_law_sis.push(si as CudaFloat);
                            gaussian_power_law_gps.push(gp);
                        }

                        FluxDensityType::CurvedPowerLaw { .. } => todo!(),

                        FluxDensityType::List { .. } => {
                            gaussian_list_radecs.push(radec);
                            gaussian_list_lmns.push(lmn);
                            gaussian_list_fds.push(comp.flux_type.clone());
                            gaussian_list_gps.push(gp);
                        }
                    };
                }

                ComponentType::Shapelet {
                    maj,
                    min,
                    pa,
                    coeffs,
                } => {
                    let gp = cuda::GaussianParams {
                        maj: *maj as CudaFloat,
                        min: *min as CudaFloat,
                        pa: *pa as CudaFloat,
                    };
                    match comp.flux_type {
                        FluxDensityType::PowerLaw { si, .. } => {
                            shapelet_power_law_radecs.push(radec);
                            shapelet_power_law_lmns.push(lmn);
                            let fd_at_150mhz = comp
                                .flux_type
                                .estimate_at_freq(cuda::POWER_LAW_FD_REF_FREQ as _);
                            let inst_fd: Jones<f64> = fd_at_150mhz.to_inst_stokes();
                            let cuda_inst_fd = jones_to_cuda_jones(inst_fd);
                            shapelet_power_law_fds.push(cuda_inst_fd);
                            shapelet_power_law_sis.push(si as CudaFloat);
                            shapelet_power_law_gps.push(gp);
                            shapelet_power_law_coeffs.push(coeffs.clone());
                        }

                        FluxDensityType::CurvedPowerLaw { .. } => todo!(),

                        FluxDensityType::List { .. } => {
                            shapelet_list_radecs.push(radec);
                            shapelet_list_lmns.push(lmn);
                            shapelet_list_fds.push(comp.flux_type.clone());
                            shapelet_list_gps.push(gp);
                            shapelet_list_coeffs.push(coeffs.clone());
                        }
                    }
                }
            }
        }

        let point_list_fds =
            get_instrumental_flux_densities(&point_list_fds, unflagged_fine_chan_freqs)
                .mapv(jones_to_cuda_jones);
        let gaussian_list_fds =
            get_instrumental_flux_densities(&gaussian_list_fds, unflagged_fine_chan_freqs)
                .mapv(jones_to_cuda_jones);
        let shapelet_list_fds =
            get_instrumental_flux_densities(&shapelet_list_fds, unflagged_fine_chan_freqs)
                .mapv(jones_to_cuda_jones);

        let (shapelet_power_law_coeffs, shapelet_power_law_coeff_lens) =
            get_flattened_coeffs(shapelet_power_law_coeffs);
        let (shapelet_list_coeffs, shapelet_list_coeff_lens) =
            get_flattened_coeffs(shapelet_list_coeffs);

        // Variables for CUDA. They're made flexible in their types for
        // whichever precision is being used in the CUDA code.
        let (unflagged_fine_chan_freqs_ints, unflagged_fine_chan_freqs_floats): (Vec<_>, Vec<_>) =
            unflagged_fine_chan_freqs
                .iter()
                .map(|&f| (f as u32, f as CudaFloat))
                .unzip();
        let shapelet_basis_values: Vec<CudaFloat> = shapelet_basis_values
            .iter()
            .map(|&f| f as CudaFloat)
            .collect();

        let n = unflagged_tile_xyzs.len();
        let num_baselines = (n * (n - 1)) / 2;

        let d_vis: DevicePointer<f32> = DevicePointer::malloc(
            num_baselines * unflagged_fine_chan_freqs.len() * std::mem::size_of::<Jones<f32>>(),
        )?;
        // Ensure the visibilities are zero'd.
        cuda_runtime_sys::cudaMemset(
            d_vis.get_mut().cast(),
            0,
            num_baselines * unflagged_fine_chan_freqs.len() * std::mem::size_of::<Jones<f32>>(),
        );
        cuda_runtime_sys::cudaDeviceSynchronize();

        let d_freqs = DevicePointer::copy_to_device(&unflagged_fine_chan_freqs_floats)?;
        let d_shapelet_basis_values = DevicePointer::copy_to_device(&shapelet_basis_values)?;

        let mut tile_index_to_unflagged_tile_index_map: Vec<i32> =
            Vec::with_capacity(unflagged_tile_xyzs.len());
        let mut i_unflagged_tile = 0;
        for i_tile in 0..unflagged_tile_xyzs.len() + tile_flags.len() {
            if tile_flags.contains(&i_tile) {
                i_unflagged_tile += 1;
                continue;
            }
            tile_index_to_unflagged_tile_index_map.push(i_unflagged_tile as i32);
            i_unflagged_tile += 1;
        }
        let d_tile_index_to_unflagged_tile_index_map =
            DevicePointer::copy_to_device(&tile_index_to_unflagged_tile_index_map)?;

        Ok(Self {
            cuda_beam: beam.prepare_cuda_beam(&unflagged_fine_chan_freqs_ints)?,
            array_latitude_rad,

            freqs: unflagged_fine_chan_freqs_floats,
            unflagged_tile_xyzs,
            tile_index_to_unflagged_tile_index_map: d_tile_index_to_unflagged_tile_index_map,

            sbf_l: sbf_l.try_into().unwrap(),
            sbf_n: sbf_n.try_into().unwrap(),
            sbf_c: sbf_c as CudaFloat,
            sbf_dx: sbf_dx as CudaFloat,

            d_vis,
            d_freqs,
            d_shapelet_basis_values,

            point_power_law_radecs,
            point_power_law_lmns: DevicePointer::copy_to_device(&point_power_law_lmns)?,
            point_power_law_fds: DevicePointer::copy_to_device(&point_power_law_fds)?,
            point_power_law_sis: DevicePointer::copy_to_device(&point_power_law_sis)?,
            point_list_radecs,
            point_list_lmns: DevicePointer::copy_to_device(&point_list_lmns)?,
            point_list_fds: DevicePointer::copy_to_device(point_list_fds.as_slice().unwrap())?,

            gaussian_power_law_radecs,
            gaussian_power_law_lmns: DevicePointer::copy_to_device(&gaussian_power_law_lmns)?,
            gaussian_power_law_fds: DevicePointer::copy_to_device(&gaussian_power_law_fds)?,
            gaussian_power_law_sis: DevicePointer::copy_to_device(&gaussian_power_law_sis)?,
            gaussian_power_law_gps: DevicePointer::copy_to_device(&gaussian_power_law_gps)?,
            gaussian_list_radecs,
            gaussian_list_lmns: DevicePointer::copy_to_device(&gaussian_list_lmns)?,
            gaussian_list_fds: DevicePointer::copy_to_device(
                gaussian_list_fds.as_slice().unwrap(),
            )?,
            gaussian_list_gps: DevicePointer::copy_to_device(&gaussian_list_gps)?,

            shapelet_power_law_radecs,
            shapelet_power_law_lmns: DevicePointer::copy_to_device(&shapelet_power_law_lmns)?,
            shapelet_power_law_fds: DevicePointer::copy_to_device(&shapelet_power_law_fds)?,
            shapelet_power_law_sis: DevicePointer::copy_to_device(&shapelet_power_law_sis)?,
            shapelet_power_law_gps: DevicePointer::copy_to_device(&shapelet_power_law_gps)?,
            shapelet_power_law_coeffs: DevicePointer::copy_to_device(&shapelet_power_law_coeffs)?,
            shapelet_power_law_coeff_lens: DevicePointer::copy_to_device(
                &shapelet_power_law_coeff_lens,
            )?,
            shapelet_list_radecs,
            shapelet_list_lmns: DevicePointer::copy_to_device(&shapelet_list_lmns)?,
            shapelet_list_fds: DevicePointer::copy_to_device(
                shapelet_list_fds.as_slice().unwrap(),
            )?,
            shapelet_list_gps: DevicePointer::copy_to_device(&shapelet_list_gps)?,
            shapelet_list_coeffs: DevicePointer::copy_to_device(&shapelet_list_coeffs)?,
            shapelet_list_coeff_lens: DevicePointer::copy_to_device(&shapelet_list_coeff_lens)?,
        })
    }

    /// Generate model visibilities for a single timestep on the GPU.
    ///
    /// # Safety
    ///
    /// This function interfaces directly with the CUDA API. Rust errors attempt
    /// to catch problems but there are no guarantees.
    // TODO: Do the minimal amount of copying, rather than copying everything
    // for every timestep.
    pub unsafe fn model_timestep(
        &self,
        mut vis_model_slice: ArrayViewMut2<Jones<f32>>,
        lst_rad: f64,
        uvws: &[UVW],
    ) -> Result<(), BeamError> {
        // Expose all the struct fields to ensure they're all used.
        let Self {
            cuda_beam,
            array_latitude_rad,

            freqs,
            unflagged_tile_xyzs: _,
            tile_index_to_unflagged_tile_index_map: _,

            sbf_l: _,
            sbf_n: _,
            sbf_c: _,
            sbf_dx: _,

            d_vis,
            d_freqs: _,
            d_shapelet_basis_values: _,

            point_power_law_radecs,
            point_power_law_lmns,
            point_power_law_fds,
            point_power_law_sis,
            point_list_radecs,
            point_list_lmns,
            point_list_fds,
            gaussian_power_law_radecs,
            gaussian_power_law_lmns,
            gaussian_power_law_fds,
            gaussian_power_law_sis,
            gaussian_power_law_gps,
            gaussian_list_radecs,
            gaussian_list_lmns,
            gaussian_list_fds,
            gaussian_list_gps,
            shapelet_power_law_radecs,
            shapelet_power_law_lmns,
            shapelet_power_law_fds,
            shapelet_power_law_sis,
            shapelet_power_law_gps,
            shapelet_power_law_coeffs,
            shapelet_power_law_coeff_lens,
            shapelet_list_radecs,
            shapelet_list_lmns,
            shapelet_list_fds,
            shapelet_list_gps,
            shapelet_list_coeffs,
            shapelet_list_coeff_lens,
        } = self;

        let cuda_uvws: Vec<cuda::UVW> = uvws
            .iter()
            .map(|&uvw| cuda::UVW {
                u: uvw.u as CudaFloat,
                v: uvw.v as CudaFloat,
                w: uvw.w as CudaFloat,
            })
            .collect();
        let d_uvws = DevicePointer::copy_to_device(&cuda_uvws)?;

        let to_azels = |x: &[RADec]| -> Vec<AzEl> {
            x.par_iter()
                .map(|radec| radec.to_hadec(lst_rad).to_azel(*array_latitude_rad))
                .collect()
        };

        if !point_power_law_radecs.is_empty() || !point_list_radecs.is_empty() {
            let point_beam_jones = {
                let mut azels = to_azels(point_power_law_radecs);
                let mut list_azels = to_azels(point_list_radecs);
                azels.append(&mut list_azels);
                cuda_beam.calc_jones(&azels)?
            };

            let cuda_status = cuda::model_points(
                &cuda::Points {
                    num_power_law_points: point_power_law_radecs.len(),
                    power_law_lmns: point_power_law_lmns.get_mut(),
                    power_law_fds: point_power_law_fds.get_mut(),
                    power_law_sis: point_power_law_sis.get_mut(),
                    num_list_points: point_list_radecs.len(),
                    list_lmns: point_list_lmns.get_mut(),
                    list_fds: point_list_fds.get_mut(),
                },
                &self.get_addresses(),
                d_uvws.get(),
                point_beam_jones.get().cast(),
            );
            let error_str =
                std::ffi::CString::from_vec_unchecked(vec![0; CUDA_ERROR_STR_LENGTH]).into_raw();
            cuda_status_to_error(cuda_status, error_str)?;
        }

        if !gaussian_power_law_radecs.is_empty() || !gaussian_list_radecs.is_empty() {
            let gaussian_beam_jones = {
                let mut azels = to_azels(gaussian_power_law_radecs);
                let mut list_azels = to_azels(gaussian_list_radecs);
                azels.append(&mut list_azels);
                cuda_beam.calc_jones(&azels)?
            };

            let cuda_status = cuda::model_gaussians(
                &cuda::Gaussians {
                    num_power_law_gaussians: gaussian_power_law_radecs.len(),
                    power_law_lmns: gaussian_power_law_lmns.get_mut(),
                    power_law_fds: gaussian_power_law_fds.get_mut(),
                    power_law_sis: gaussian_power_law_sis.get_mut(),
                    power_law_gps: gaussian_power_law_gps.get_mut(),
                    num_list_gaussians: gaussian_list_radecs.len(),
                    list_lmns: gaussian_list_lmns.get_mut(),
                    list_fds: gaussian_list_fds.get_mut(),
                    list_gps: gaussian_list_gps.get_mut(),
                },
                &self.get_addresses(),
                d_uvws.get(),
                gaussian_beam_jones.get().cast(),
            );
            let error_str =
                std::ffi::CString::from_vec_unchecked(vec![0; CUDA_ERROR_STR_LENGTH]).into_raw();
            cuda_status_to_error(cuda_status, error_str)?;
        }

        if !shapelet_power_law_radecs.is_empty() || !shapelet_list_radecs.is_empty() {
            let shapelet_beam_jones = {
                let mut azels = to_azels(shapelet_power_law_radecs);
                let mut list_azels = to_azels(shapelet_list_radecs);
                azels.append(&mut list_azels);
                cuda_beam.calc_jones(&azels)?
            };

            let uvs = self.get_shapelet_uvs(lst_rad);
            let power_law_uvs = DevicePointer::copy_to_device(uvs.power_law.as_slice().unwrap())?;
            let list_uvs = DevicePointer::copy_to_device(uvs.list.as_slice().unwrap())?;

            let cuda_status = cuda::model_shapelets(
                &cuda::Shapelets {
                    num_power_law_shapelets: shapelet_power_law_radecs.len(),
                    power_law_lmns: shapelet_power_law_lmns.get_mut(),
                    power_law_fds: shapelet_power_law_fds.get_mut(),
                    power_law_sis: shapelet_power_law_sis.get_mut(),
                    power_law_gps: shapelet_power_law_gps.get_mut(),
                    power_law_shapelet_uvs: power_law_uvs.get_mut(),
                    power_law_shapelet_coeffs: shapelet_power_law_coeffs.get_mut(),
                    power_law_num_shapelet_coeffs: shapelet_power_law_coeff_lens.get_mut(),
                    num_list_shapelets: shapelet_list_radecs.len(),
                    list_lmns: shapelet_list_lmns.get_mut(),
                    list_fds: shapelet_list_fds.get_mut(),
                    list_gps: shapelet_list_gps.get_mut(),
                    list_shapelet_uvs: list_uvs.get_mut(),
                    list_shapelet_coeffs: shapelet_list_coeffs.get_mut(),
                    list_num_shapelet_coeffs: shapelet_list_coeff_lens.get_mut(),
                },
                &self.get_addresses(),
                d_uvws.get(),
                shapelet_beam_jones.get().cast(),
            );
            let error_str =
                std::ffi::CString::from_vec_unchecked(vec![0; CUDA_ERROR_STR_LENGTH]).into_raw();
            cuda_status_to_error(cuda_status, error_str)?;
        }

        // Rust's strict typing means that we can't neatly call
        // `copy_from_device` on `d_vis` into `vis_model_slice`. Do the copy
        // manually.
        cuda_runtime_sys::cudaMemcpy(
            vis_model_slice.as_mut_ptr().cast(),
            d_vis.get().cast(),
            uvws.len() * freqs.len() * std::mem::size_of::<Jones<f32>>(),
            cuda_runtime_sys::cudaMemcpyKind::cudaMemcpyDeviceToHost,
        );
        // Clear the device visibilities.
        cuda_runtime_sys::cudaMemset(
            d_vis.get_mut().cast(),
            0,
            uvws.len() * freqs.len() * std::mem::size_of::<Jones<f32>>(),
        );
        cuda_runtime_sys::cudaDeviceSynchronize();

        Ok(())
    }

    /// Get a populated [cuda::Addresses]. This should never outlive `self`.
    fn get_addresses(&self) -> cuda::Addresses {
        let n = self.unflagged_tile_xyzs.len();
        let num_baselines = (n * (n - 1)) / 2;

        cuda::Addresses {
            num_freqs: self.freqs.len() as _,
            num_vis: (num_baselines * self.freqs.len()) as _,
            num_tiles: n as _,
            sbf_l: self.sbf_l,
            sbf_n: self.sbf_n,
            sbf_c: self.sbf_c,
            sbf_dx: self.sbf_dx,
            d_freqs: self.d_freqs.get_mut(),
            d_shapelet_basis_values: self.d_shapelet_basis_values.get_mut(),
            num_unique_beam_freqs: self.cuda_beam.get_num_unique_freqs(),
            d_tile_map: self.cuda_beam.get_tile_map(),
            d_freq_map: self.cuda_beam.get_freq_map(),
            d_tile_index_to_unflagged_tile_index_map: self
                .tile_index_to_unflagged_tile_index_map
                .get(),
            d_vis: self.d_vis.get_mut().cast(),
        }
    }

    /// Shapelets need their own special kind of UVW coordinates. Each shapelet
    /// component's position is treated as the phase centre. This function uses
    /// the FFI type [cuda::ShapeletUV]; the W isn't actually used in
    /// computation, and omitting it is hopefully a little more efficient.
    ///
    /// The returned arrays have baseline as the first axis and component as the
    /// second.
    fn get_shapelet_uvs(&self, lst_rad: f64) -> ShapeletUVs {
        ShapeletUVs {
            power_law: get_shapelet_uvs_inner(
                &self.shapelet_power_law_radecs,
                lst_rad,
                self.unflagged_tile_xyzs,
            ),
            list: get_shapelet_uvs_inner(
                &self.shapelet_list_radecs,
                lst_rad,
                self.unflagged_tile_xyzs,
            ),
        }
    }
}

impl std::fmt::Debug for SkyModellerCuda<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SkyModellerCuda").finish()
    }
}

/// The return type of [SkyModellerCuda::get_shapelet_uvs]. These arrays have
/// baseline as the first axis and component as the second.
struct ShapeletUVs {
    power_law: Array2<cuda::ShapeletUV>,
    list: Array2<cuda::ShapeletUV>,
}

fn get_shapelet_uvs_inner(
    radecs: &[RADec],
    lst_rad: f64,
    tile_xyzs: &[XyzGeodetic],
) -> Array2<cuda::ShapeletUV> {
    let n = tile_xyzs.len();
    let num_baselines = (n * (n - 1)) / 2;

    let mut shapelet_uvs: Array2<cuda::ShapeletUV> = Array2::from_elem(
        (num_baselines, radecs.len()),
        cuda::ShapeletUV { u: 0.0, v: 0.0 },
    );
    shapelet_uvs
        .axis_iter_mut(Axis(1))
        .into_par_iter()
        .zip(radecs.par_iter())
        .for_each(|(mut baseline_uv, radec)| {
            let hadec = radec.to_hadec(lst_rad);
            let shapelet_uvs: Vec<cuda::ShapeletUV> = xyzs_to_cross_uvws_parallel(tile_xyzs, hadec)
                .into_iter()
                .map(|uvw| cuda::ShapeletUV {
                    u: uvw.u as CudaFloat,
                    v: uvw.v as CudaFloat,
                })
                .collect();
            baseline_uv.assign(&Array1::from(shapelet_uvs));
        });
    shapelet_uvs
}

/// There are a variable number of shapelet coefficients for each shapelet
/// component. To avoid excessive dereferencing on GPUs (expensive), this
/// method flattens the coefficients into a single array (lengths of the
/// array-of-arrays).
fn get_flattened_coeffs(
    shapelet_coeffs: Vec<Vec<ShapeletCoeff>>,
) -> (Vec<cuda::ShapeletCoeff>, Vec<usize>) {
    let mut coeffs: Vec<cuda::ShapeletCoeff> = vec![];
    let mut coeff_lengths = Vec::with_capacity(coeffs.len());

    for coeffs_for_comp in shapelet_coeffs {
        coeff_lengths.push(coeffs_for_comp.len());
        for coeff in coeffs_for_comp {
            coeffs.push(cuda::ShapeletCoeff {
                n1: coeff.n1,
                n2: coeff.n2,
                value: coeff.value as CudaFloat,
            })
        }
    }

    coeffs.shrink_to_fit();
    (coeffs, coeff_lengths)
}
