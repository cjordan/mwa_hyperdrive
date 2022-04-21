// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Code for FEE beam calculations.

use std::path::{Path, PathBuf};

use log::trace;
use marlu::{AzEl, Jones};
use ndarray::prelude::*;

use super::{Beam, BeamError, BeamType, Delays};
use mwa_hyperdrive_common::{cfg_if, log, marlu, ndarray};

cfg_if::cfg_if! {
    if #[cfg(feature = "cuda")] {
        use marlu::cuda::*;
        use super::{BeamCUDA, CudaFloat};
    }
}

/// A wrapper of the `FEEBeam` struct in hyperbeam that implements the [Beam]
/// trait.
pub(super) struct FEEBeam {
    hyperbeam_object: mwa_hyperbeam::fee::FEEBeam,
    delays: Array2<u32>,
    gains: Array2<f64>,
    file: PathBuf,
}

impl FEEBeam {
    fn new_inner(
        hyperbeam_object: mwa_hyperbeam::fee::FEEBeam,
        num_tiles: usize,
        delays: Delays,
        gains: Option<Array2<f64>>,
        file: Option<&Path>,
    ) -> Result<FEEBeam, BeamError> {
        let delays = match delays {
            Delays::Full(d) => d,
            Delays::Partial(d) => partial_to_full(d, num_tiles),
        };

        // If no gains were provided, assume all are alive.
        let gains = match gains {
            Some(g) => g,
            None => Array2::ones(delays.dim()),
        };

        // Complain if the dimensions of delays and gains don't match.
        // TODO: Tidy.
        if delays.dim().0 != gains.dim().0 {
            return Err(BeamError::DelayGainsDimensionMismatch {
                delays: delays.dim().0,
                gains: gains.dim().0,
            });
        }

        // Wrap the `FEEBeam` out of hyperbeam with our own `FEEBeam`.
        Ok(FEEBeam {
            hyperbeam_object,
            delays,
            gains,
            file: match file {
                Some(p) => p.to_path_buf(),
                None => PathBuf::from(std::env::var("MWA_BEAM_FILE").unwrap()),
            },
        })
    }

    fn new(
        file: &Path,
        num_tiles: usize,
        delays: Delays,
        gains: Option<Array2<f64>>,
    ) -> Result<FEEBeam, BeamError> {
        let hyperbeam_object = mwa_hyperbeam::fee::FEEBeam::new(file)?;
        Self::new_inner(hyperbeam_object, num_tiles, delays, gains, Some(file))
    }

    pub(super) fn new_from_env(
        num_tiles: usize,
        delays: Delays,
        gains: Option<Array2<f64>>,
    ) -> Result<FEEBeam, BeamError> {
        let hyperbeam_object = mwa_hyperbeam::fee::FEEBeam::new_from_env()?;
        Self::new_inner(hyperbeam_object, num_tiles, delays, gains, None)
    }

    fn calc_jones_inner(
        &self,
        azel: AzEl,
        freq_hz: f64,
        delays: &[u32],
        amps: &[f64],
    ) -> Result<Jones<f64>, mwa_hyperbeam::fee::FEEBeamError> {
        self.hyperbeam_object
            .calc_jones(azel.az, azel.za(), freq_hz as _, delays, amps, true)
            // Convert hyperbeam's `Jones` to our `Jones`. Both come from Marlu,
            // but the Marlu used all over hyperdrive is imported differently
            // from that of hyperbeam. Testing shows that this operation takes
            // tens of nanoseconds.
            .map(|j| unsafe { std::mem::transmute(j) })
    }

    fn calc_jones_array_inner(
        &self,
        azels: &[AzEl],
        freq_hz: f64,
        delays: &[u32],
        amps: &[f64],
    ) -> Result<Vec<Jones<f64>>, mwa_hyperbeam::fee::FEEBeamError> {
        let (azs, zas): (Vec<f64>, Vec<f64>) =
            azels.iter().map(|azel| (azel.az, azel.za())).unzip();
        self.hyperbeam_object
            .calc_jones_array(&azs, &zas, freq_hz as _, delays, amps, true)
            // Convert hyperbeam's `Jones` to our `Jones`. Both come from Marlu,
            // but the Marlu used all over hyperdrive is imported differently
            // from that of hyperbeam. Testing shows that this operation takes
            // tens of nanoseconds.
            .map(|v| unsafe { std::mem::transmute(v) })
    }
}

impl Beam for FEEBeam {
    fn get_beam_type(&self) -> BeamType {
        BeamType::FEE
    }

    fn get_num_tiles(&self) -> usize {
        self.delays.len_of(Axis(0))
    }

    fn get_beam_file(&self) -> Option<&Path> {
        Some(&self.file)
    }

    fn calc_jones(
        &self,
        azel: AzEl,
        freq_hz: f64,
        tile_index: usize,
    ) -> Result<Jones<f64>, BeamError> {
        // The FEE beam is defined only at specific frequencies. For this
        // reason, rather than making a unique hash for every single different
        // frequency, round specified frequency (`freq_hz`) to the nearest beam
        // frequency and use that for the hash.
        let beam_freq = self.find_closest_freq(freq_hz);

        // TODO: Check tile_index isn't too big.
        let delays = self.delays.slice(s![tile_index, ..]);
        let amps = self.gains.slice(s![tile_index, ..]);
        let jones = self.calc_jones_inner(
            azel,
            beam_freq,
            delays.as_slice().unwrap(),
            amps.as_slice().unwrap(),
        )?;
        Ok(jones)
    }

    fn calc_jones_array(
        &self,
        azels: &[AzEl],
        freq_hz: f64,
        tile_index: usize,
    ) -> Result<Vec<Jones<f64>>, BeamError> {
        // The FEE beam is defined only at specific frequencies. For this
        // reason, rather than making a unique hash for every single different
        // frequency, round specified frequency (`freq_hz`) to the nearest beam
        // frequency and use that for the hash.
        let beam_freq = self.find_closest_freq(freq_hz);

        let delays = self.delays.slice(s![tile_index, ..]);
        let amps = self.gains.slice(s![tile_index, ..]);
        self.calc_jones_array_inner(
            azels,
            beam_freq,
            delays.as_slice().unwrap(),
            amps.as_slice().unwrap(),
        )
        .map_err(BeamError::from)
    }

    fn find_closest_freq(&self, desired_freq_hz: f64) -> f64 {
        self.hyperbeam_object
            .find_closest_freq(desired_freq_hz as _) as _
    }

    fn empty_coeff_cache(&self) {
        self.hyperbeam_object.empty_cache();
    }

    #[cfg(feature = "cuda")]
    unsafe fn prepare_cuda_beam(&self, freqs_hz: &[u32]) -> Result<Box<dyn BeamCUDA>, BeamError> {
        let cuda_beam = self.hyperbeam_object.cuda_prepare(
            freqs_hz,
            self.delays.view(),
            self.gains.view(),
            true,
        )?;
        Ok(Box::new(FEEBeamCUDA {
            hyperbeam_object: cuda_beam,
        }))
    }
}

#[cfg(feature = "cuda")]
pub struct FEEBeamCUDA {
    hyperbeam_object: mwa_hyperbeam::fee::FEEBeamCUDA,
}

#[cfg(feature = "cuda")]
impl BeamCUDA for FEEBeamCUDA {
    unsafe fn calc_jones(
        &self,
        azels: &[AzEl],
    ) -> Result<DevicePointer<Jones<CudaFloat>>, BeamError> {
        let (azs, zas): (Vec<CudaFloat>, Vec<CudaFloat>) = azels
            .iter()
            .map(|&azel| (azel.az as CudaFloat, azel.za() as CudaFloat))
            .unzip();
        self.hyperbeam_object
            .calc_jones_device(&azs, &zas, true)
            // This hilariously unsafe map is to convert hyperbeam's `Jones` to
            // our `Jones`. Both come from Marlu, but the Marlu used all over
            // hyperdrive is imported differently from that of hyperbeam.
            // Testing shows that this operation takes tens of nanoseconds.
            .map(|ptr| unsafe { std::mem::transmute(ptr) })
            .map_err(BeamError::from)
    }

    fn get_beam_type(&self) -> BeamType {
        BeamType::FEE
    }

    fn get_tile_map(&self) -> *const i32 {
        self.hyperbeam_object.get_tile_map()
    }

    fn get_freq_map(&self) -> *const i32 {
        self.hyperbeam_object.get_freq_map()
    }

    fn get_num_unique_freqs(&self) -> i32 {
        self.hyperbeam_object.get_num_unique_freqs()
    }
}

/// Create an FEE beam object. The dipole delays and amps (a.k.a. gains) must
/// have the same number of rows; these correspond to individual tiles.
///
/// `amps` *must* have 16 or 32 elements per row (each corresponds to an MWA
/// dipole in a tile, in the M&C order; see
/// <https://wiki.mwatelescope.org/pages/viewpage.action?pageId=48005139>).
pub fn create_fee_beam_object<P: AsRef<Path>>(
    beam_file: Option<P>,
    num_tiles: usize,
    delays: Delays,
    gains: Option<Array2<f64>>,
) -> Result<Box<dyn Beam>, BeamError> {
    trace!("Setting up FEE beam");
    let beam = if let Some(bf) = beam_file {
        // Set up the FEE beam struct from the specified beam file.
        Box::new(FEEBeam::new(bf.as_ref(), num_tiles, delays, gains)?)
    } else {
        // Set up the FEE beam struct from the MWA_BEAM_FILE environment
        // variable.
        Box::new(FEEBeam::new_from_env(num_tiles, delays, gains)?)
    };
    Ok(beam)
}

/// Assume that the dipole delays for all tiles is the same as the delays for
/// one tile.
fn partial_to_full(delays: Vec<u32>, num_tiles: usize) -> Array2<u32> {
    let mut out = Array2::zeros((num_tiles, 16));
    let d = Array1::from(delays);
    out.outer_iter_mut().for_each(|mut tile_delays| {
        tile_delays.assign(&d);
    });
    out
}