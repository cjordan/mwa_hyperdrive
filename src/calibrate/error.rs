// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Error type for all calibration-related errors.

use thiserror::Error;

use super::{params::InvalidArgsError, solutions::WriteSolutionsError};
use crate::data_formats::{
    uvfits::{UvfitsReadError, UvfitsWriteError},
    ReadInputDataError,
};
use mwa_hyperdrive_core::{mwalib, EstimateError};
use mwalib::fitsio;

#[derive(Error, Debug)]
pub enum CalibrateError {
    #[error("{0}")]
    InvalidArgs(#[from] InvalidArgsError),

    #[error("{0}")]
    Estimate(#[from] EstimateError),

    #[error("{0}")]
    Read(#[from] ReadInputDataError),

    #[error("{0}")]
    WriteSolutions(#[from] WriteSolutionsError),

    #[error("{0}")]
    Beam(#[from] crate::beam::BeamError),

    #[error("cfitsio error: {0}")]
    Fitsio(#[from] fitsio::errors::Error),

    #[error("Error when reading uvfits: {0}")]
    UviftsRead(#[from] UvfitsReadError),

    #[error("Error when writing uvfits: {0}")]
    UviftsWrite(#[from] UvfitsWriteError),

    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
}
