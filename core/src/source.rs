// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

/*!
Structures to describe sky-model sources and their components.
 */

use serde::{Deserialize, Serialize};

use crate::flux_density::EstimateError;
use crate::*;

use rayon::prelude::*;

#[derive(Clone, Debug, PartialEq)]
/// A collection of components.
pub struct Source {
    /// The components associated with the source.
    pub components: Vec<SourceComponent>,
}

impl Source {
    /// Calculate the (l,m,n) coordinates of each component's (RA,Dec). The
    /// calculation is done in parallel.
    pub fn get_lmn(&self, pointing: &RADec) -> Vec<LMN> {
        self.components
            .par_iter()
            .map(|comp| comp.radec.to_lmn(&pointing))
            .collect()
    }

    /// Estimate the flux densities for each of a source's components given a
    /// frequency.
    pub fn get_flux_estimates(&self, freq_hz: f64) -> Result<Vec<FluxDensity>, EstimateError> {
        self.components
            .iter()
            .map(|comp| comp.flux_type.estimate_at_freq(freq_hz))
            .collect()
    }

    /// Estimate the flux densities for each of a source's components given a
    /// frequency. The calculation is done in parallel.
    pub fn get_flux_estimates_parallel(
        &self,
        freq_hz: f64,
    ) -> Result<Vec<FluxDensity>, EstimateError> {
        self.components
            .par_iter()
            .map(|comp| comp.flux_type.estimate_at_freq(freq_hz))
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq)]
/// Information on a source's component.
pub struct SourceComponent {
    /// Coordinates struct associated with the component.
    pub radec: RADec,
    /// The type of component.
    pub comp_type: ComponentType,
    /// The flux densities associated with this component.
    pub flux_type: FluxDensityType,
}

/// Source types supported by hyperdrive.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ComponentType {
    #[serde(rename = "point")]
    Point,

    #[serde(rename = "gaussian")]
    Gaussian {
        /// Major axis size [radians]
        maj: f64,
        /// Minor axis size [radians]
        min: f64,
        /// Position angle [radians]
        pa: f64,
    },

    #[serde(rename = "shapelet")]
    Shapelet {
        /// Major axis size [radians]
        maj: f64,
        /// Minor axis size [radians]
        min: f64,
        /// Position angle [radians]
        pa: f64,
        /// Shapelet coefficients
        coeffs: Vec<ShapeletCoeff>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShapeletCoeff {
    pub n1: u8,
    pub n2: u8,
    pub coeff: f64,
}
