// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Poly-phase filter bank gains.
//!
//! These can be thought of an attenuation applied to MWA visibilities. Undoing
//! the attenuation helps "flatten" the spectral bandpass of visibilities and
//! aids calibration. The gains are the same for each coarse channel (regardless
//! of its frequency).

// TODO: Provide gains for all frequency resolutions.
// TOOD: Get gains from Jake.

use itertools::Itertools;
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter, EnumString};

use mwa_hyperdrive_common::{itertools, lazy_static};

/// All available kinds of PFB gains.
#[derive(Debug, Display, Clone, Copy, EnumIter, EnumString)]
pub(crate) enum PfbFlavour {
    /// Use the "RTS empirical" gains.
    #[strum(serialize = "empirical")]
    Empirical,

    /// Use the "Alan Levine" gains.
    #[strum(serialize = "levine")]
    Levine,

    /// Don't apply any PFB gains.
    #[strum(serialize = "none")]
    None,
}

pub(super) const DEFAULT_PFB_FLAVOUR: PfbFlavour = PfbFlavour::Empirical;

lazy_static::lazy_static! {
    // Useful for help texts.
    pub(super) static ref PFB_FLAVOURS: String = PfbFlavour::iter().join(", ");
}

/// Gains from empirical averaging of RTS BP solution points using "Anish" PFB
/// gains for 1062363808 and backing out corrections to flatten average coarse
/// channel. Taken from RTS source code.
pub(crate) const EMPIRICAL_40KHZ: [f64; 32] = [
    0.5,
    0.5,
    0.67874855,
    0.83576969,
    0.95187049,
    1.0229769,
    1.05711736,
    1.06407012,
    1.06311151,
    1.06089592,
    1.0593481,
    1.06025714,
    1.06110822,
    1.05893943,
    1.05765503,
    1.05601938,
    1.056496995, // This value for the centre channel was originally 0.5 (a placeholder as its usually flaggec), but MWAX data can use the centre channel, so it has been changed to be the average of its neighbours.
    1.05697461,
    1.05691842,
    1.05688129,
    1.05623901,
    1.05272663,
    1.05272112,
    1.05551337,
    1.05724941,
    1.0519857,
    1.02483081,
    0.96454596,
    0.86071928,
    0.71382954,
    0.5,
    0.5,
];

/// Alan Levine's gains from PFB simulations. Taken from RTS source code.
pub(crate) const LEVINE_40KHZ: [f64; 32] = [
    0.5173531193404733,
    0.5925143901943901,
    0.7069509925949563,
    0.8246794181334419,
    0.9174323810107883,
    0.9739924923371597,
    0.9988235178442829,
    1.0041872682882493,
    1.0021295484391897,
    1.0000974383045906,
    1.0004197495080835,
    1.002092702099684,
    1.003201858357689,
    1.0027668031914465,
    1.001305418352239,
    1.0001674256814668,
    1.0003506058381628,
    1.001696297529349,
    1.0030147335641364,
    1.0030573420014388,
    1.0016582119173054,
    1.0001394672444315,
    1.0004004241051296,
    1.002837790192105,
    1.0039523509152424,
    0.9949679743767017,
    0.9632053940967067,
    0.8975113804877556,
    0.7967436134595853,
    0.6766433460480191,
    0.5686988482410316,
    0.5082890508180502,
];
