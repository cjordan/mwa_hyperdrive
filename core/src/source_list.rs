// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

/*!
Code surrounding the `BTreeMap` used to contain all sky-model sources and their
components.
 */

use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};

use crate::*;

use rayon::prelude::*;

/// A `BTreeMap` of source names for keys and `Source` structs for values.
///
/// By making `SourceList` a new type (specifically, an anonymous struct),
/// useful methods can be put onto it.
#[derive(Debug, Clone, Default)]
pub struct SourceList(BTreeMap<String, Source>);

impl SourceList {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get azimuth and zenith angle coordinates for all components of all
    /// sources. Useful for interfacing with beam code. The sources are iterated
    /// in parallel.
    ///
    /// Because `SourceList` is a `BTreeMap`, the order of the sources is always
    /// the same, so the (Az, ZA) coordinates returned from this function are
    /// 1:1 with sources and their components.
    pub fn get_azza(&self, lst_rad: f64, latitude_rad: f64) -> (Vec<f64>, Vec<f64>) {
        let mut all_az = vec![];
        let mut all_za = vec![];
        let nested_az_za: Vec<(Vec<f64>, Vec<f64>)> = self
            .par_iter()
            // For each source, get all of its component's (Az, ZA) coordinates.
            .map(|(_, src)| {
                let (az, za): (Vec<_>, Vec<_>) = src
                    .components
                    .iter()
                    .map(|comp| {
                        let azel = comp.radec.to_hadec(lst_rad).to_azel(latitude_rad);
                        // Unpack the `AzEl` struct and make a tuple of azimuth and
                        // zenith angle.
                        (azel.az, azel.za())
                    })
                    .unzip();
                (az, za)
            })
            .collect();

        for (mut az, mut za) in nested_az_za {
            all_az.append(&mut az);
            all_za.append(&mut za);
        }

        (all_az, all_za)
    }

    /// Get azimuth and zenith angle coordinates for all components of all
    /// sources, assuming that the latitude is the MWA's latitude. See the
    /// documentation for `SourceList::get_azza` for more details.
    pub fn get_azza_mwa(&self, lst_rad: f64) -> (Vec<f64>, Vec<f64>) {
        Self::get_azza(&self, lst_rad, crate::constants::MWA_LAT_RAD)
    }
}

impl Deref for SourceList {
    type Target = BTreeMap<String, Source>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SourceList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl IntoIterator for SourceList {
    type Item = (String, Source);
    type IntoIter = std::collections::btree_map::IntoIter<String, Source>;

    #[inline]
    fn into_iter(self) -> std::collections::btree_map::IntoIter<String, Source> {
        self.0.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::*;
    use ndarray::arr1;
    use std::f64::consts::*;

    #[test]
    // Test that the (Az, ZA) coordinates retrieved from the `.get_azza()`
    // method of `SourceList` are correct and always in the same order.
    fn test_get_azza_mwa() {
        let mut sl = SourceList::new();
        // Use a common component. Only the `radec` part needs to be modified.
        let comp = SourceComponent {
            radec: RADec::new(PI, FRAC_PI_4),
            comp_type: ComponentType::Point,
            flux_type: FluxDensityType::PowerLaw {
                si: -0.8,
                fd: FluxDensity {
                    freq: 100.0,
                    i: 10.0,
                    q: 7.0,
                    u: 6.0,
                    v: 1.0,
                },
            },
        };
        let mut s = Source { components: vec![] };

        // Don't modify the first component.
        s.components.push(comp.clone());

        // Modify the coordinates of other components.
        s.components.push(comp.clone());
        s.components.last_mut().unwrap().radec = RADec::new(PI - 0.1, FRAC_PI_4 + 0.1);

        s.components.push(comp.clone());
        s.components.last_mut().unwrap().radec = RADec::new(PI + 0.1, FRAC_PI_4 - 0.1);

        // Push "source_1".
        sl.insert("source_1".to_string(), s);

        let mut s = Source { components: vec![] };
        s.components.push(comp.clone());

        s.components.push(comp.clone());
        s.components.last_mut().unwrap().radec = RADec::new(PI - 0.1, FRAC_PI_4 + 0.1);

        s.components.push(comp.clone());
        s.components.last_mut().unwrap().radec = RADec::new(PI + 0.1, FRAC_PI_4 - 0.1);

        sl.insert("source_2".to_string(), s);

        let mut s = Source { components: vec![] };
        s.components.push(comp.clone());
        s.components.last_mut().unwrap().radec = RADec::new(FRAC_PI_2, PI);

        s.components.push(comp.clone());
        s.components.last_mut().unwrap().radec = RADec::new(FRAC_PI_2 - 0.1, PI + 0.2);

        sl.insert("source_3".to_string(), s);

        let lst = 3.0 * FRAC_PI_4;
        let (az, za) = sl.get_azza_mwa(lst);
        let az_expected = [
            0.5284641294204054,
            0.4140207507698987,
            0.6516588664580675,
            0.5284641294204054,
            0.4140207507698987,
            0.6516588664580675,
            1.9931268490084542,
            2.1121964836053806,
        ];
        let za_expected = [
            1.4415169467014715,
            1.4807939480563403,
            1.416863456467004,
            1.4415169467014715,
            1.4807939480563403,
            1.416863456467004,
            2.254528351516936,
            2.0543439118454256,
        ];
        assert_abs_diff_eq!(arr1(&az), arr1(&az_expected), epsilon = 1e-10);
        assert_abs_diff_eq!(arr1(&za), arr1(&za_expected), epsilon = 1e-10);
    }
}