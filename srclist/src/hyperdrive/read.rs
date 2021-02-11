// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

/*!
Code to read in hyperdrive source lists.

To make the source list files a little easier to read and write, `SourceList`
isn't directly serialisable or deserialisable. Use temporary types here to do
the serde magic and give the caller a `SourceList`.
 */

use super::*;

fn source_list_from_tmp_sl(
    mut tmp_sl: BTreeMap<String, Vec<TmpComponent>>,
) -> Result<SourceList, ReadSourceListError> {
    let mut sl = SourceList::new();
    for (name, tmp_comps) in tmp_sl.iter_mut() {
        let mut comps = Vec::with_capacity(tmp_comps.len());

        // Ensure that the sum of each Stokes' flux densities is positive.
        let mut sum_i = 0.0;
        let mut sum_q = 0.0;
        let mut sum_u = 0.0;
        let mut sum_v = 0.0;

        for tmp_comp in tmp_comps {
            // Validation and conversion.
            if tmp_comp.ra < 0.0 || tmp_comp.ra > 360.0 {
                return Err(ReadSourceListError::InvalidRa(tmp_comp.ra));
            }
            if tmp_comp.dec < -90.0 || tmp_comp.dec > 90.0 {
                return Err(ReadSourceListError::InvalidDec(tmp_comp.dec));
            }
            let radec = RADec::new_degrees(tmp_comp.ra, tmp_comp.dec);

            let comp_type = match &mut tmp_comp.comp_type {
                ComponentType::Point => ComponentType::Point,
                ComponentType::Gaussian { maj, min, pa } => ComponentType::Gaussian {
                    maj: maj.to_radians() / 3600.0,
                    min: min.to_radians() / 3600.0,
                    pa: pa.to_radians(),
                },
                ComponentType::Shapelet {
                    maj,
                    min,
                    pa,
                    coeffs,
                } => ComponentType::Shapelet {
                    maj: maj.to_radians() / 3600.0,
                    min: min.to_radians() / 3600.0,
                    pa: pa.to_radians(),
                    coeffs: coeffs.clone(),
                },
            };

            let flux_type = match &tmp_comp.flux_type {
                TmpFluxDensityType::List(fds) => {
                    for fd in fds {
                        sum_i += fd.i;
                        sum_q += fd.q;
                        sum_u += fd.u;
                        sum_v += fd.v;
                    }
                    FluxDensityType::List { fds: fds.clone() }
                }

                TmpFluxDensityType::PowerLaw { si, fd } => {
                    sum_i += fd.i;
                    sum_q += fd.q;
                    sum_u += fd.u;
                    sum_v += fd.v;
                    FluxDensityType::PowerLaw { si: *si, fd: *fd }
                }

                TmpFluxDensityType::CurvedPowerLaw { si, fd, q } => {
                    sum_i += fd.i;
                    sum_q += fd.q;
                    sum_u += fd.u;
                    sum_v += fd.v;
                    FluxDensityType::CurvedPowerLaw {
                        si: *si,
                        fd: *fd,
                        q: *q,
                    }
                }
            };

            comps.push(SourceComponent {
                radec,
                comp_type,
                flux_type,
            })
        }

        if sum_i < 0.0 {
            return Err(ReadSourceListError::InvalidFluxDensitySum {
                sum: sum_i,
                stokes_comp: "I".to_string(),
                source_name: name.clone(),
            });
        } else if sum_q < 0.0 {
            return Err(ReadSourceListError::InvalidFluxDensitySum {
                sum: sum_q,
                stokes_comp: "Q".to_string(),
                source_name: name.clone(),
            });
        } else if sum_u < 0.0 {
            return Err(ReadSourceListError::InvalidFluxDensitySum {
                sum: sum_u,
                stokes_comp: "U".to_string(),
                source_name: name.clone(),
            });
        } else if sum_v < 0.0 {
            return Err(ReadSourceListError::InvalidFluxDensitySum {
                sum: sum_v,
                stokes_comp: "V".to_string(),
                source_name: name.clone(),
            });
        }

        sl.insert(name.clone(), Source { components: comps });
    }

    Ok(sl)
}

/// Convert a yaml file to a `SourceList`.
pub fn source_list_from_yaml<T: std::io::BufRead>(
    buf: &mut T,
) -> Result<SourceList, ReadSourceListError> {
    let tmp_sl: BTreeMap<String, Vec<TmpComponent>> = serde_yaml::from_reader(buf)?;
    source_list_from_tmp_sl(tmp_sl)
}

/// Convert a json file to a `SourceList`.
pub fn source_list_from_json<T: std::io::BufRead>(
    buf: &mut T,
) -> Result<SourceList, ReadSourceListError> {
    let tmp_sl: BTreeMap<String, Vec<TmpComponent>> = serde_json::from_reader(buf)?;
    source_list_from_tmp_sl(tmp_sl)
}
