// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Calibration software for the Murchison Widefield Array (MWA) radio
//! telescope.
//!
//! <https://mwatelescope.github.io/mwa_hyperdrive/index.html>

pub mod averaging;
pub mod beam;
mod cli;
pub(crate) mod constants;
pub(crate) mod context;
pub mod di_calibrate;
pub(crate) mod error;
pub(crate) mod filenames;
pub(crate) mod flagging;
mod help_texts;
pub(crate) mod io;
pub(crate) mod math;
pub(crate) mod messages;
pub(crate) mod metafits;
pub(crate) mod misc;
pub mod model;
pub(crate) mod solutions;
pub mod srclist;
pub(crate) mod unit_parsing;

#[cfg(feature = "cuda")]
pub(crate) mod cuda;

#[cfg(test)]
mod tests;

// Re-exports.
pub use cli::{
    di_calibrate::DiCalArgs,
    dipole_gains::DipoleGainsArgs,
    solutions::{
        apply::SolutionsApplyArgs, convert::SolutionsConvertArgs, plot::SolutionsPlotArgs,
    },
    srclist::{
        by_beam::SrclistByBeamArgs, convert::SrclistConvertArgs, shift::SrclistShiftArgs,
        verify::SrclistVerifyArgs,
    },
    vis_utils::{simulate::VisSimulateArgs, subtract::VisSubtractArgs},
};
pub use context::Polarisations;
pub use error::HyperdriveError;
pub use io::read::{
    AutoData, CrossData, MsReader, RawDataCorrections, RawDataReader, UvfitsReader,
};
pub use math::TileBaselineFlags;
pub use solutions::CalibrationSolutions;
