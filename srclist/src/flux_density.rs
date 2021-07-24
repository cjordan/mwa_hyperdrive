// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Flux density structures.

use ndarray::{Array1, Array2};
use num::Complex;
use serde::{Deserialize, Serialize};

use crate::constants::*;
use mwa_hyperdrive_core::Jones;

const LIST_TO_POWER_LAW_MAX_DIFF: f64 = 0.01; // 1%

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
/// At a frequency, four flux densities for each Stokes parameter.
// When serialising/deserialising, ignore Stokes Q U V if they are zero.
pub struct FluxDensity {
    /// The frequency that these flux densities apply to \[Hz\]
    pub freq: f64,

    /// The flux density of Stokes I \[Jy\]
    pub i: f64,

    /// The flux density of Stokes Q \[Jy\]
    #[serde(default)]
    #[serde(skip_serializing_if = "is_zero")]
    pub q: f64,

    /// The flux density of Stokes U \[Jy\]
    #[serde(default)]
    #[serde(skip_serializing_if = "is_zero")]
    pub u: f64,

    /// The flux density of Stokes V \[Jy\]
    #[serde(default)]
    #[serde(skip_serializing_if = "is_zero")]
    pub v: f64,
}

impl FluxDensity {
    /// Given two flux densities, calculate the spectral index that fits them.
    pub fn calc_spec_index(&self, fd2: &Self) -> f64 {
        (fd2.i / self.i).ln() / (fd2.freq / self.freq).ln()
    }
}

impl From<FluxDensity> for Jones<f64> {
    fn from(fd: FluxDensity) -> Self {
        Self::from([
            Complex::new(fd.i + fd.q, 0.0),
            Complex::new(fd.u, fd.v),
            Complex::new(fd.u, -fd.v),
            Complex::new(fd.i - fd.q, 0.0),
        ])
    }
}

impl From<&FluxDensity> for Jones<f64> {
    fn from(fd: &FluxDensity) -> Self {
        Self::from([
            Complex::new(fd.i + fd.q, 0.0),
            Complex::new(fd.u, fd.v),
            Complex::new(fd.u, -fd.v),
            Complex::new(fd.i - fd.q, 0.0),
        ])
    }
}

/// This is only used for serialisation
// https://stackoverflow.com/questions/53900612/how-do-i-avoid-generating-json-when-serializing-a-value-that-is-null-or-a-defaul
#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_zero(num: &f64) -> bool {
    num.abs() < f64::EPSILON
}

impl std::ops::Mul<f64> for FluxDensity {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self {
        FluxDensity {
            freq: self.freq,
            i: self.i * rhs,
            q: self.q * rhs,
            u: self.u * rhs,
            v: self.v * rhs,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum FluxDensityType {
    /// $S_\nu = a \nu^{\alpha}$
    PowerLaw {
        /// Spectral index (alpha)
        si: f64,
        /// Flux density (a)
        fd: FluxDensity,
    },

    /// Similar to a power law. See Callingham et al. 2017, section 4.1.
    ///
    /// S_\nu = a \nu^{\alpha} e^{q(\ln{v})^2}
    CurvedPowerLaw {
        /// Spectral index (alpha)
        si: f64,
        /// Flux density (a)
        fd: FluxDensity,
        /// Spectral curvature (q)
        q: f64,
    },

    /// A list of flux densities specified at multiple frequencies.
    /// Interpolation/extrapolation is needed to get flux densities at
    /// non-specified frequencies.
    List { fds: Vec<FluxDensity> },
}

impl FluxDensityType {
    /// Given flux density information, estimate the flux density at a
    /// particular frequency. For power laws / curved power laws, the "ratio" of
    /// the reference frequency and the specified frequencies is used to scale
    /// the reference flux density.
    ///
    /// If enum variant is FluxDensityType::List, then the entries must be
    /// sorted by frequency (which should be the case if the source list was
    /// read by hyperdrive). The estimated flux density is based off of the
    /// Stokes I component, so any other Stokes parameters may be poorly
    /// estimated.
    pub fn estimate_at_freq(&self, freq_hz: f64) -> FluxDensity {
        match self {
            FluxDensityType::PowerLaw { si, fd } => {
                let ratio = calc_flux_ratio(freq_hz, fd.freq, *si);
                let mut new_fd = fd.clone() * ratio;
                new_fd.freq = freq_hz;
                new_fd
            }

            FluxDensityType::CurvedPowerLaw { si, fd, q } => {
                let mut power_law_component = fd.clone() * calc_flux_ratio(freq_hz, fd.freq, *si);
                power_law_component.freq = freq_hz;
                let curved_component = (q * (freq_hz / fd.freq).ln().powi(2)).exp();
                power_law_component * curved_component
            }

            FluxDensityType::List { fds } => {
                if fds.is_empty() {
                    panic!("Tried to estimate a flux density for a component, but it had no flux densities");
                }

                let mut old_freq = -1.0;

                // `smaller_flux_density` is a bad name given to the component's flux
                // density corresponding to a frequency smaller than but nearest to the
                // specified frequency.
                let (spec_index, smaller_flux_density) = {
                    // If there's only one source component, then we must assume the
                    // spectral index for extrapolation.
                    if fds.len() == 1 {
                        // trace!("Only one flux density in a component's list; extrapolating with spectral index {}", DEFAULT_SPEC_INDEX);
                        (DEFAULT_SPEC_INDEX, &fds[0])
                    }
                    // Otherwise, find the frequencies that bound the given frequency. As we
                    // assume that the input source components are sorted by frequency, we
                    // can assume that the comp. with the smallest frequency is at index 0,
                    // and similarly for the last component.
                    else {
                        let mut smaller_comp_index: usize = 0;
                        let mut larger_comp_index: usize = fds.len() - 1;
                        for (i, fd) in fds.iter().enumerate() {
                            let f = fd.freq;
                            // Bail if this frequency is smaller than the old
                            // frequency; we require the list of flux densities
                            // to be sorted by frequency.
                            if f < old_freq {
                                panic!("The list of flux densities used for estimation were not sorted");
                            }
                            old_freq = f;

                            // Iterate until we hit a catalogue frequency bigger than the
                            // desired frequency.

                            // If this freq and the specified freq are the same...
                            if (f - freq_hz).abs() < 1e-3 {
                                // ... then just return the flux density information from
                                // this frequency.
                                return fd.clone();
                            }
                            // If this freq is smaller than the specified freq...
                            else if f < freq_hz {
                                // Check if we've iterated to the last catalogue component -
                                // if so, then we must extrapolate (i.e. the specified
                                // freq. is bigger than all catalogue frequencies).
                                if i == fds.len() - 1 {
                                    smaller_comp_index = fds.len() - 2;
                                }
                            }
                            // We only arrive here if f > freq.
                            else {
                                // Because the array is sorted, we now know the closest two
                                // frequencies. The only exception is if this is the first
                                // catalogue frequency (i.e. the desired freq is smaller
                                // than all catalogue freqs -> extrapolate).
                                if i == 0 {
                                    larger_comp_index = 1;
                                } else {
                                    smaller_comp_index = i - 1;
                                }
                                break;
                            }
                        }

                        let mut spec_index =
                            fds[smaller_comp_index].calc_spec_index(&fds[larger_comp_index]);

                        // Stop stupid spectral indices.
                        if spec_index < SPEC_INDEX_CAP {
                            // trace!(
                            //     "Component had a spectral index {}; capping at {}",
                            //     spec_index,
                            //     SPEC_INDEX_CAP
                            // );
                            spec_index = SPEC_INDEX_CAP;
                        }

                        (
                            spec_index,
                            // If our last component's frequency is smaller than the specified
                            // freq., then we should use that for flux densities.
                            if fds[larger_comp_index].freq < freq_hz {
                                &fds[larger_comp_index]
                            } else {
                                &fds[smaller_comp_index]
                            },
                        )
                    }
                };

                // Now scale the flux densities given the calculated spectral index.
                let flux_ratio = calc_flux_ratio(freq_hz, smaller_flux_density.freq, spec_index);

                FluxDensity {
                    freq: freq_hz,
                    ..*smaller_flux_density
                } * flux_ratio
            }
        }
    }

    /// If certain conditions are met, convert a [FluxDensityType::List] to
    /// [FluxDensityType::PowerLaw]. Specifically: if there are one or two flux
    /// densities in the list, or the list flux densities track a power law well
    /// enough.
    ///
    /// This function assumes that the list flux densities are ascendingly
    /// sorted by frequency and that there is always at least one flux density.
    pub fn convert_list_to_power_law(&mut self) {
        match self {
            FluxDensityType::List { fds } => {
                match fds.len() {
                    0 => panic!(
                    "convert_list_to_power_law got a FluxDensityType without any flux densities!"
                ),
                    1 => {
                        *self = FluxDensityType::PowerLaw {
                            si: DEFAULT_SPEC_INDEX,
                            fd: fds[0].clone(),
                        }
                    }
                    2 => {
                        let si = fds[0].calc_spec_index(&fds[1]);
                        *self = FluxDensityType::PowerLaw {
                            si,
                            fd: fds[0].clone(),
                        };
                    }
                    // Lengths > 2.
                    _ => {
                        // Solving for the spectral index and reference flux
                        // density.

                        // S = S_0 \nu ^ \alpha
                        // Arrange this as Ax = b
                        // ln(S) = ln(S_0) + \alpha * ln(\nu)
                        // x_0 = \alpha
                        // x_1 = ln(S_0)
                        // A_i = [ln(\nu_i), 1]
                        // b = ln(S)

                        let mut a = Array2::ones((fds.len(), 2));
                        a.outer_iter_mut()
                            .zip(fds.iter())
                            .for_each(|(mut row, fd)| {
                                // Just use the Stokes I values.
                                row[[0]] = fd.freq.ln();
                                // We don't need to set the other value because that's
                                // already one.
                            });
                        let b = Array1::from(fds.iter().map(|fd| fd.i.ln()).collect::<Vec<_>>());

                        // A^T A (call this C)
                        let at = a.t();
                        let c = at.dot(&a).into_raw_vec();

                        // (A^T A)^-1
                        // Fortunately, no fancy linalg is needed because A^T A is
                        // always 2x2 here.
                        let det = c[0] * c[3] - c[1] * c[2];
                        if det > 10000.0 {
                            // Bad determinant; abort.
                            return;
                        }
                        let c = Array2::from_shape_vec((2, 2), vec![c[3], -c[1], -c[2], c[0]])
                            .unwrap()
                            / det;

                        // x = (A^T A)^-1 A^T b
                        let x = c.dot(&at).dot(&b).to_vec();

                        // Form a new FluxDensityType and account for the other
                        // Stokes parameters.
                        let middle = fds.len() / 2;
                        let fd = &fds[middle];
                        let i = x[1].exp() * fd.freq.powf(x[0]);
                        let ratio = i / fd.i;

                        let new_fdt = FluxDensityType::PowerLaw {
                            si: x[0],
                            fd: FluxDensity {
                                freq: fd.freq,
                                i,
                                q: fd.q * ratio,
                                u: fd.u * ratio,
                                v: fd.v * ratio,
                            },
                        };

                        // How well does this new power law compare to the original
                        // data?
                        for i in 0..fds.len() {
                            let old = &fds[i];
                            let new = new_fdt.estimate_at_freq(old.freq);
                            // If any of the differences are too big, the flux
                            // densities should not be modelled as a power law.
                            if ((new.i - old.i) / new.i).abs() > LIST_TO_POWER_LAW_MAX_DIFF
                                || ((new.q - old.q) / new.q).abs() > LIST_TO_POWER_LAW_MAX_DIFF
                                || ((new.u - old.u) / new.u).abs() > LIST_TO_POWER_LAW_MAX_DIFF
                                || ((new.v - old.v) / new.v).abs() > LIST_TO_POWER_LAW_MAX_DIFF
                            {
                                return;
                            }
                        }

                        // Overwrite the input list with a power law.
                        *self = new_fdt;
                    }
                }
            }

            // All other enum variants aren't handled by this function.
            _ => (),
        }
    }
}

/// Given a spectral index, determine the flux-density ratio of two frequencies.
pub fn calc_flux_ratio(desired_freq_hz: f64, cat_freq_hz: f64, spec_index: f64) -> f64 {
    (desired_freq_hz / cat_freq_hz).powf(spec_index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use mwa_hyperdrive_core::c64;

    #[test]
    fn calc_freq_ratio_1() {
        let desired_freq = 160.0;
        let cat_freq = 150.0;
        let spec_index = -0.6;
        let ratio = calc_flux_ratio(desired_freq, cat_freq, spec_index);
        let expected = 0.9620170425907598;
        assert_abs_diff_eq!(ratio, expected, epsilon = 1e-10);
    }

    #[test]
    fn calc_freq_ratio_2() {
        let desired_freq = 140.0;
        let cat_freq = 150.0;
        let spec_index = -0.6;
        let ratio = calc_flux_ratio(desired_freq, cat_freq, spec_index);
        let expected = 1.0422644718599143;
        assert_abs_diff_eq!(ratio, expected, epsilon = 1e-10);
    }

    fn get_fdt() -> FluxDensityType {
        // J034844-125505 from
        // srclist_pumav3_EoR0aegean_fixedEoR1pietro+ForA_phase1+2.txt
        FluxDensityType::List {
            fds: vec![
                FluxDensity {
                    freq: 120e6,
                    i: 8.00841,
                    ..Default::default()
                },
                FluxDensity {
                    freq: 140e6,
                    i: 6.80909,
                    ..Default::default()
                },
                FluxDensity {
                    freq: 160e6,
                    i: 5.91218,
                    ..Default::default()
                },
                FluxDensity {
                    freq: 180e6,
                    i: 5.21677,
                    ..Default::default()
                },
            ],
        }
    }

    #[test]
    #[should_panic]
    fn test_none_convert_list_to_power_law() {
        let mut fdt = get_fdt();
        // This is definitely a list.
        assert!(matches!(fdt, FluxDensityType::List { .. }));
        // Empty the flux densities. Our function will panic.
        match &mut fdt {
            FluxDensityType::List { fds } => *fds = vec![],
            _ => unreachable!(),
        }
        fdt.convert_list_to_power_law();
    }

    #[test]
    fn test_one_convert_list_to_power_law() {
        let mut fdt = get_fdt();
        // This is definitely a list.
        assert!(matches!(fdt, FluxDensityType::List { .. }));
        // Leave one flux density.
        match &mut fdt {
            FluxDensityType::List { fds } => *fds = vec![fds[0].clone()],
            _ => unreachable!(),
        }
        fdt.convert_list_to_power_law();
        // It's been converted to a power law.
        assert!(matches!(fdt, FluxDensityType::PowerLaw { .. }));
        // We're using the default SI.
        match fdt {
            FluxDensityType::PowerLaw { si, .. } => assert_abs_diff_eq!(si, DEFAULT_SPEC_INDEX),
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_two_convert_list_to_power_law() {
        let mut fdt = get_fdt();
        // This is definitely a list.
        assert!(matches!(fdt, FluxDensityType::List { .. }));
        // Leave two flux densities.
        match &mut fdt {
            FluxDensityType::List { fds } => *fds = vec![fds[0].clone(), fds[1].clone()],
            _ => unreachable!(),
        }
        fdt.convert_list_to_power_law();
        // It's been converted to a power law.
        assert!(matches!(fdt, FluxDensityType::PowerLaw { .. }));
        // We're using the SI between the only two FDs.
        match fdt {
            FluxDensityType::PowerLaw { si, .. } => assert_abs_diff_eq!(si, -1.0524361973093983),
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_many_convert_list_to_power_law() {
        let mut fdt = get_fdt();
        // This is definitely a list.
        assert!(matches!(fdt, FluxDensityType::List { .. }));
        fdt.convert_list_to_power_law();
        // It's been converted to a power law.
        assert!(matches!(fdt, FluxDensityType::PowerLaw { .. }));
        // We're using the SI between the middle two FDs.
        match fdt {
            FluxDensityType::PowerLaw { si, fd } => {
                assert_abs_diff_eq!(si, -1.0570227720845136);
                assert_abs_diff_eq!(fd.freq, 160e6);
                assert_abs_diff_eq!(fd.i, 5.910484034862892);
                assert_abs_diff_eq!(fd.q, 0.0);
                assert_abs_diff_eq!(fd.u, 0.0);
                assert_abs_diff_eq!(fd.v, 0.0);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_to_jones() {
        let fd = FluxDensity {
            freq: 170e6,
            i: 0.058438801501144624,
            q: -0.3929914018344019,
            u: -0.3899498110659575,
            v: -0.058562589895788,
        };
        let result = Jones::from(&fd);
        assert_abs_diff_eq!(result[0], c64::new(fd.i + fd.q, 0.0));
        assert_abs_diff_eq!(result[1], c64::new(fd.u, fd.v));
        assert_abs_diff_eq!(result[2], c64::new(fd.u, -fd.v));
        assert_abs_diff_eq!(result[3], c64::new(fd.i - fd.q, 0.0));
    }

    #[test]
    fn test_to_jones_borrowed() {
        let fd = FluxDensity {
            freq: 170e6,
            i: 0.058438801501144624,
            q: -0.3929914018344019,
            u: -0.3899498110659575,
            v: -0.058562589895788,
        };
        let result = Jones::from(&fd);
        assert_abs_diff_eq!(result[0], c64::new(fd.i + fd.q, 0.0));
        assert_abs_diff_eq!(result[1], c64::new(fd.u, fd.v));
        assert_abs_diff_eq!(result[2], c64::new(fd.u, -fd.v));
        assert_abs_diff_eq!(result[3], c64::new(fd.i - fd.q, 0.0));
    }
}
