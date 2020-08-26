// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

/*!
Parsing of RTS source lists.

RTS sources always have a "base source", which can be thought of as a
non-optional component. Coordinates are hour angle and declination, which have
units of decimal hours (i.e. 0 - 24) and degrees, respectively.

Gaussian and shapelet sizes are specified in arcminutes, whereas position angles
are in degrees. All frequencies are in Hz, and all flux densities are in Jy.

All flux densities are specified in the "list" style, and all have units of Jy.

Keywords like SOURCE, COMPONENT, POINT etc. must be at the start of a line (i.e.
no preceeding space).
 */

use log::warn;

use super::*;

/// Parse a buffer containing an RTS-style source list into a `SourceList`.
pub fn parse_source_list<T: std::io::BufRead>(
    buf: &mut T,
) -> Result<SourceList, ReadSourceListError> {
    let mut line = String::new();
    let mut line_num: u32 = 0;
    let mut in_source = false;
    let mut in_component = false;
    let mut in_shapelet = false;
    let mut in_shapelet2 = false;
    let mut found_shapelets = vec![];
    let mut component_type_set = false;
    let mut source_name = String::new();
    let mut components: Vec<SourceComponent> = vec![];
    let mut source_list: SourceList = BTreeMap::new();

    let parse_float = |string: &str, line_num: u32| -> Result<f64, ReadSourceListCommonError> {
        string
            .parse()
            .map_err(|_| ReadSourceListCommonError::ParseFloatError {
                line_num,
                string: string.to_string(),
            })
    };

    let float_to_int = |float: f64, line_num: u32| -> Result<u8, ReadSourceListCommonError> {
        if float < 0.0 || float > std::u8::MAX as _ {
            Err(ReadSourceListCommonError::FloatToIntError { line_num, float })
        } else {
            Ok(float as u8)
        }
    };

    while buf.read_line(&mut line).expect("IO error") > 0 {
        line_num += 1;

        // Handle lines that aren't intended to parsed (comments and blank
        // lines).
        if line.starts_with('#') | line.starts_with('\n') {
            line.clear();
            continue;
        }
        // We ignore any lines starting with whitespace, but emit a warning.
        else if line.starts_with(' ') | line.starts_with('\t') {
            warn!(
                "Source list line {} starts with whitespace; ignoring it",
                line_num
            );
            line.clear();
            continue;
        }

        let mut items = line.split_whitespace();
        match items.next() {
            Some("SOURCE") => {
                if in_source {
                    return Err(ReadSourceListCommonError::NestedSources(line_num).into());
                } else {
                    in_source = true;

                    // SOURCE lines must have at least 4 elements (including SOURCE).
                    match items.next() {
                        Some(name) => source_name.push_str(name),
                        None => {
                            return Err(
                                ReadSourceListCommonError::IncompleteSourceLine(line_num).into()
                            )
                        }
                    };
                    let hour_angle = match items.next() {
                        Some(ha) => parse_float(ha, line_num)?,
                        None => {
                            return Err(
                                ReadSourceListCommonError::IncompleteSourceLine(line_num).into()
                            )
                        }
                    };
                    let declination = match items.next() {
                        Some(dec) => parse_float(dec, line_num)?,
                        None => {
                            return Err(
                                ReadSourceListCommonError::IncompleteSourceLine(line_num).into()
                            )
                        }
                    };
                    if items.next().is_some() {
                        warn!(
                            "Source list line {}: Ignoring trailing contents after declination",
                            line_num
                        );
                    }

                    // Validation and conversion.
                    if hour_angle < 0.0 || hour_angle > 24.0 {
                        return Err(ReadSourceListError::InvalidHa(hour_angle));
                    }
                    if declination < -90.0 || declination > 90.0 {
                        return Err(ReadSourceListError::InvalidDec(declination));
                    }
                    let radec = RADec::new(hour_angle * DH2R, declination.to_radians());

                    components.push(SourceComponent {
                        radec,
                        // Assume the base source is a point source. If we find
                        // component type information, we can overwrite this.
                        comp_type: ComponentType::Point,
                        flux_type: FluxDensityType::List { fds: vec![] },
                    });
                }
            }

            // Flux density line.
            Some("FREQ") => {
                if !in_source {
                    return Err(ReadSourceListCommonError::OutsideSource {
                        line_num,
                        keyword: "FREQ".to_string(),
                    }
                    .into());
                }

                // FREQ lines must have at least 6 elements (including FREQ).
                let freq = match items.next() {
                    Some(f) => parse_float(f, line_num)?,
                    None => {
                        return Err(ReadSourceListCommonError::IncompleteFluxLine(line_num).into())
                    }
                };
                let stokes_i = match items.next() {
                    Some(f) => parse_float(f, line_num)?,
                    None => {
                        return Err(ReadSourceListCommonError::IncompleteFluxLine(line_num).into())
                    }
                };
                let stokes_q = match items.next() {
                    Some(f) => parse_float(f, line_num)?,
                    None => {
                        return Err(ReadSourceListCommonError::IncompleteFluxLine(line_num).into())
                    }
                };
                let stokes_u = match items.next() {
                    Some(f) => parse_float(f, line_num)?,
                    None => {
                        return Err(ReadSourceListCommonError::IncompleteFluxLine(line_num).into())
                    }
                };
                let stokes_v = match items.next() {
                    Some(f) => parse_float(f, line_num)?,
                    None => {
                        return Err(ReadSourceListCommonError::IncompleteFluxLine(line_num).into())
                    }
                };
                if items.next().is_some() {
                    warn!(
                        "Source list line {}: Ignoring trailing contents after Stokes V",
                        line_num
                    );
                }

                let fd = FluxDensity {
                    freq,
                    i: stokes_i,
                    q: stokes_q,
                    u: stokes_u,
                    v: stokes_v,
                };

                match components.iter_mut().last().map(|c| &mut c.flux_type) {
                    Some(FluxDensityType::List { fds }) => fds.push(fd),
                    _ => unreachable!(),
                }
            }

            // Gaussian component type.
            Some("GAUSSIAN") => {
                if !in_source {
                    return Err(ReadSourceListCommonError::OutsideSource {
                        line_num,
                        keyword: "GAUSSIAN".to_string(),
                    }
                    .into());
                }

                // GAUSSIAN lines must have at least 4 elements (including
                // GAUSSIAN).
                let position_angle = match items.next() {
                    Some(f) => parse_float(f, line_num)?,
                    None => {
                        return Err(ReadSourceListRtsError::IncompleteGaussianLine(line_num).into())
                    }
                };
                let maj_arcmin = match items.next() {
                    Some(f) => parse_float(f, line_num)?,
                    None => {
                        return Err(ReadSourceListRtsError::IncompleteGaussianLine(line_num).into())
                    }
                };
                let min_arcmin = match items.next() {
                    Some(f) => parse_float(f, line_num)?,
                    None => {
                        return Err(ReadSourceListRtsError::IncompleteGaussianLine(line_num).into())
                    }
                };
                if items.next().is_some() {
                    warn!(
                        "Source list line {}: Ignoring trailing contents after minor axis",
                        line_num
                    );
                }

                let comp_type = ComponentType::Gaussian {
                    maj: (maj_arcmin / 60.0).to_radians(),
                    min: (min_arcmin / 60.0).to_radians(),
                    pa: position_angle.to_radians(),
                };

                // Have we already set the component type?
                if component_type_set {
                    return Err(ReadSourceListCommonError::MultipleComponentTypes(line_num).into());
                }
                components.iter_mut().last().unwrap().comp_type = comp_type;
                component_type_set = true;
            }

            // Shapelet component type.
            Some("SHAPELET2") => {
                if !in_source {
                    return Err(ReadSourceListCommonError::OutsideSource {
                        line_num,
                        keyword: "SHAPELET2".to_string(),
                    }
                    .into());
                }

                // SHAPELET2 lines must have at least 4 elements (including
                // SHAPELET2).
                let position_angle = match items.next() {
                    Some(f) => parse_float(f, line_num)?,
                    None => {
                        return Err(ReadSourceListRtsError::IncompleteShapelet2Line(line_num).into())
                    }
                };
                let maj_arcmin = match items.next() {
                    Some(f) => parse_float(f, line_num)?,
                    None => {
                        return Err(ReadSourceListRtsError::IncompleteShapelet2Line(line_num).into())
                    }
                };
                let min_arcmin = match items.next() {
                    Some(f) => parse_float(f, line_num)?,
                    None => {
                        return Err(ReadSourceListRtsError::IncompleteShapelet2Line(line_num).into())
                    }
                };
                if items.next().is_some() {
                    warn!(
                        "Source list line {}: Ignoring trailing contents after minor axis",
                        line_num
                    );
                }

                let comp_type = ComponentType::Shapelet {
                    maj: (maj_arcmin / 60.0).to_radians(),
                    min: (min_arcmin / 60.0).to_radians(),
                    pa: position_angle.to_radians(),
                    coeffs: vec![],
                };

                // Have we already set the component type?
                if component_type_set {
                    return Err(ReadSourceListCommonError::MultipleComponentTypes(line_num).into());
                }
                components.iter_mut().last().unwrap().comp_type = comp_type;
                component_type_set = true;
                in_shapelet2 = true;
            }

            // "Jenny's shapelet type" - not to be used any more.
            Some("SHAPELET") => {
                if !in_source {
                    return Err(ReadSourceListCommonError::OutsideSource {
                        line_num,
                        keyword: "SHAPELET".to_string(),
                    }
                    .into());
                }

                // Make the component type a "dummy shapelet component",
                // identifiable by its position angle of -999.
                let comp_type = ComponentType::Shapelet {
                    maj: 0.0,
                    min: 0.0,
                    pa: -999.0,
                    coeffs: vec![],
                };
                components.iter_mut().last().unwrap().comp_type = comp_type;

                warn!("Source list line {}: Ignoring SHAPELET component", line_num);
                component_type_set = true;
                in_shapelet = true;
            }

            // Shapelet coefficient.
            Some("COEFF") => {
                if !in_source {
                    return Err(ReadSourceListCommonError::OutsideSource {
                        line_num,
                        keyword: "COEFF".to_string(),
                    }
                    .into());
                }

                // Did we parse a SHAPELET or SHAPELET2 line earlier?
                if !in_shapelet && !in_shapelet2 {
                    return Err(ReadSourceListRtsError::MissingShapeletLine(line_num).into());
                }

                // COEFF lines must have at least 4 elements (including COEFF).
                let n1 = match items.next() {
                    Some(f) => parse_float(f, line_num)?,
                    None => {
                        return Err(ReadSourceListRtsError::IncompleteCoeffLine(line_num).into())
                    }
                };
                let n2 = match items.next() {
                    Some(f) => parse_float(f, line_num)?,
                    None => {
                        return Err(ReadSourceListRtsError::IncompleteCoeffLine(line_num).into())
                    }
                };
                let coeff = match items.next() {
                    Some(f) => parse_float(f, line_num)?,
                    None => {
                        return Err(ReadSourceListRtsError::IncompleteCoeffLine(line_num).into())
                    }
                };
                if items.next().is_some() {
                    warn!(
                        "Source list line {}: Ignoring trailing contents after minor axis",
                        line_num
                    );
                }

                // Because we ignore SHAPELET components, only add this COEFF
                // line if we're dealing with a SHAPELET2.
                if in_shapelet2 {
                    let shapelet_coeff = ShapeletCoeff {
                        n1: float_to_int(n1, line_num)?,
                        n2: float_to_int(n2, line_num)?,
                        coeff,
                    };
                    match &mut components.iter_mut().last().unwrap().comp_type {
                        ComponentType::Shapelet { coeffs, .. } => coeffs.push(shapelet_coeff),
                        _ => {
                            return Err(ReadSourceListRtsError::MissingShapeletLine(line_num).into())
                        }
                    }
                }
            }

            // New component.
            Some("COMPONENT") => {
                if !in_source {
                    return Err(ReadSourceListCommonError::OutsideSource {
                        line_num,
                        keyword: "COMPONENT".to_string(),
                    }
                    .into());
                }

                // Before we start working with a new component, check that the
                // last component struct added actually has flux densities (this
                // struct corresponds to the "base source"). RTS source lists
                // can only have the "list" type.
                match &components.last().unwrap().flux_type {
                    FluxDensityType::List { fds } => {
                        if fds.is_empty() {
                            return Err(ReadSourceListCommonError::NoFluxDensities(line_num).into());
                        }
                    }
                    _ => unreachable!(),
                }

                if in_component {
                    return Err(ReadSourceListCommonError::NestedComponents(line_num).into());
                }
                in_component = true;

                // COMPONENT lines must have at least 3 elements (including COMPONENT).
                let hour_angle = match items.next() {
                    Some(ha) => parse_float(ha, line_num)?,
                    None => {
                        return Err(ReadSourceListCommonError::IncompleteSourceLine(line_num).into())
                    }
                };
                let declination = match items.next() {
                    Some(dec) => parse_float(dec, line_num)?,
                    None => {
                        return Err(ReadSourceListCommonError::IncompleteSourceLine(line_num).into())
                    }
                };
                if items.next().is_some() {
                    warn!(
                        "Source list line {}: Ignoring trailing contents after declination",
                        line_num
                    );
                }

                components.push(SourceComponent {
                    radec: RADec::new(hour_angle * DH2R, declination.to_radians()),
                    // Assume the base source is a point source. If we find
                    // component type information, we can overwrite this.
                    comp_type: ComponentType::Point,
                    flux_type: FluxDensityType::List { fds: vec![] },
                });

                in_shapelet = false;
                in_shapelet2 = false;
                component_type_set = false;
            }

            Some("ENDCOMPONENT") => {
                if !in_component {
                    return Err(ReadSourceListCommonError::EarlyEndComponent(line_num).into());
                }

                // Check that the last component struct added actually has flux
                // densities. RTS source lists can only have the "list" type.
                match &mut components.iter_mut().last().unwrap().flux_type {
                    FluxDensityType::List { fds } => {
                        if fds.is_empty() {
                            return Err(ReadSourceListCommonError::NoFluxDensities(line_num).into());
                        } else {
                            // Sort the existing flux densities by frequency.
                            fds.sort_unstable_by(|&a, &b| {
                                a.freq.partial_cmp(&b.freq).unwrap_or_else(|| {
                                    panic!("Couldn't compare {} to {}", a.freq, b.freq)
                                })
                            });
                        }
                    }
                    _ => unreachable!(),
                }

                // If we were reading a SHAPELET2 component, check that
                // shapelet coefficients were read.
                if in_shapelet2 {
                    match &components.last().unwrap().comp_type {
                        ComponentType::Shapelet { coeffs, .. } => {
                            if coeffs.is_empty() {
                                return Err(
                                    ReadSourceListRtsError::NoShapeletCoeffs(line_num).into()
                                );
                            }
                        }
                        _ => unreachable!(),
                    }
                }

                in_component = false;
                in_shapelet = false;
                in_shapelet2 = false;
                component_type_set = false;
            }

            Some("ENDSOURCE") => {
                if !in_source {
                    return Err(ReadSourceListCommonError::EarlyEndSource(line_num).into());
                } else if in_component {
                    return Err(ReadSourceListCommonError::MissingEndComponent(line_num).into());
                }
                let mut source = Source { components: vec![] };
                source.components.append(&mut components);

                // Find any SHAPELET components (not SHAPELET2 components). If
                // we find one, we ignore it, and we don't need to return an
                // error if this source has no components. Also ensure that the
                // sum of each Stokes' flux densities is positive.
                let mut sum_i = 0.0;
                let mut sum_q = 0.0;
                let mut sum_u = 0.0;
                let mut sum_v = 0.0;
                for (i, c) in source.components.iter().enumerate() {
                    if let ComponentType::Shapelet { pa, .. } = c.comp_type {
                        if (pa + 999.0).abs() < 1e-3 {
                            found_shapelets.push(i);
                        }
                    }

                    match &c.flux_type {
                        FluxDensityType::List { fds } => {
                            for fd in fds {
                                sum_i += fd.i;
                                sum_q += fd.q;
                                sum_u += fd.u;
                                sum_v += fd.v;
                            }
                        }

                        FluxDensityType::PowerLaw { fd, .. } => {
                            sum_i += fd.i;
                            sum_q += fd.q;
                            sum_u += fd.u;
                            sum_v += fd.v;
                        }

                        FluxDensityType::CurvedPowerLaw { fd, .. } => {
                            sum_i += fd.i;
                            sum_q += fd.q;
                            sum_u += fd.u;
                            sum_v += fd.v;
                        }
                    }
                }
                if sum_i < 0.0 {
                    return Err(ReadSourceListError::InvalidFluxDensitySum {
                        sum: sum_i,
                        stokes_comp: "I".to_string(),
                        source_name,
                    });
                } else if sum_q < 0.0 {
                    return Err(ReadSourceListError::InvalidFluxDensitySum {
                        sum: sum_q,
                        stokes_comp: "Q".to_string(),
                        source_name,
                    });
                } else if sum_u < 0.0 {
                    return Err(ReadSourceListError::InvalidFluxDensitySum {
                        sum: sum_u,
                        stokes_comp: "U".to_string(),
                        source_name,
                    });
                } else if sum_v < 0.0 {
                    return Err(ReadSourceListError::InvalidFluxDensitySum {
                        sum: sum_v,
                        stokes_comp: "V".to_string(),
                        source_name,
                    });
                }

                // Delete any found shapelets.
                for &i in &found_shapelets {
                    source.components.remove(i);
                }

                if source.components.is_empty() && found_shapelets.is_empty() {
                    return Err(ReadSourceListCommonError::NoComponents(line_num).into());
                }

                // Check that the last component struct added actually has flux
                // densities. RTS source lists can only have the "list" type.
                if !source.components.is_empty() {
                    match &mut source.components.iter_mut().last().unwrap().flux_type {
                        FluxDensityType::List { fds } => {
                            if fds.is_empty() {
                                return Err(
                                    ReadSourceListCommonError::NoFluxDensities(line_num).into()
                                );
                            } else {
                                // Sort the existing flux densities by frequency.
                                fds.sort_unstable_by(|&a, &b| {
                                    a.freq.partial_cmp(&b.freq).unwrap_or_else(|| {
                                        panic!("Couldn't compare {} to {}", a.freq, b.freq)
                                    })
                                });
                            }
                        }
                        _ => unreachable!(),
                    }
                }

                // If we were reading a SHAPELET2 component, check that
                // shapelet coefficients were read.
                if in_shapelet2 {
                    match &source.components.last().unwrap().comp_type {
                        ComponentType::Shapelet { coeffs, .. } => {
                            if coeffs.is_empty() {
                                return Err(
                                    ReadSourceListRtsError::NoShapeletCoeffs(line_num).into()
                                );
                            }
                        }
                        _ => unreachable!(),
                    }
                }

                // If we found SHAPELET components, but now there are no
                // components left, don't add this source to the source list.
                if !source.components.is_empty() {
                    source_list.insert(source_name.clone(), source);
                }

                in_source = false;
                in_shapelet = false;
                in_shapelet2 = false;
                component_type_set = false;
                source_name.clear();
            }

            Some(k) => {
                return Err(ReadSourceListCommonError::UnrecognisedKeyword {
                    line_num,
                    keyword: k.to_string(),
                }
                .into())
            }

            // Empty line, continue.
            None => (),
        }

        line.clear(); // clear to reuse the buffer line.
    }

    // If we're still "in a source", but we've finished reading lines, then an
    // ENDSOURCE must be missing.
    if in_source {
        return Err(ReadSourceListCommonError::MissingEndSource(line_num).into());
    }

    // Complain if no sources were read.
    if source_list.is_empty() {
        return Err(ReadSourceListCommonError::NoSources(line_num).into());
    }

    Ok(source_list)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use approx::*;
    // indoc allows us to write test source lists that look like they would in a
    // file.
    use indoc::indoc;

    use super::*;

    #[test]
    fn parse_source_1() {
        let mut sl = Cursor::new(indoc! {"
        SOURCE VLA_ForA 3.40182 -37.5551
        FREQ 180e+6 1.0 0 0 0
        ENDSOURCE
        SOURCE VLA_ForB 3.40166 -37.0551
        FREQ 180e+6 2.0 0 0 0
        ENDSOURCE
        "});

        let result = parse_source_list(&mut sl);
        assert!(result.is_ok(), "{:?}", result);
        let sl = result.unwrap();
        assert_eq!(sl.len(), 2);

        assert!(sl.contains_key("VLA_ForA"));
        let s = sl.get("VLA_ForA").unwrap();
        assert_eq!(s.components.len(), 1);
        let comp = &s.components[0];
        assert_abs_diff_eq!(comp.radec.ra, 3.40182 * DH2R);
        assert_abs_diff_eq!(comp.radec.dec, -37.5551_f64.to_radians());
        assert!(match comp.flux_type {
            FluxDensityType::List { .. } => true,
            _ => false,
        });
        let fds = match &comp.flux_type {
            FluxDensityType::List { fds } => fds,
            _ => unreachable!(),
        };
        assert_abs_diff_eq!(fds[0].freq, 180e6);
        assert_abs_diff_eq!(fds[0].i, 1.0);
        assert_abs_diff_eq!(fds[0].q, 0.0);

        assert!(sl.contains_key("VLA_ForB"));
        let s = sl.get("VLA_ForB").unwrap();
        assert_eq!(s.components.len(), 1);
        let comp = &s.components[0];
        assert_abs_diff_eq!(comp.radec.ra, 3.40166 * DH2R);
        assert_abs_diff_eq!(comp.radec.dec, -37.0551_f64.to_radians());
        assert!(match comp.flux_type {
            FluxDensityType::List { .. } => true,
            _ => false,
        });
        let fds = match &comp.flux_type {
            FluxDensityType::List { fds } => fds,
            _ => unreachable!(),
        };
        assert_abs_diff_eq!(fds[0].freq, 180e6);
        assert_abs_diff_eq!(fds[0].i, 2.0);
        assert_abs_diff_eq!(fds[0].q, 0.0);
    }

    #[test]
    fn parse_source_1_no_trailing_newline() {
        let mut sl = Cursor::new(indoc! {"
        SOURCE VLA_ForA 3.40182 -37.5551
        FREQ 180e+6 1.0 0 0 0
        ENDSOURCE
        SOURCE VLA_ForB 3.40166 -37.0551
        FREQ 180e+6 2.0 0 0 0
        ENDSOURCE"});

        let result = parse_source_list(&mut sl);
        assert!(result.is_ok(), "{:?}", result);
        let sl = result.unwrap();
        assert_eq!(sl.len(), 2);

        assert!(sl.contains_key("VLA_ForA"));
        assert!(sl.contains_key("VLA_ForB"));
    }

    #[test]
    fn parse_source_2() {
        let mut sl = Cursor::new(indoc! {"
        SOURCE VLA_ForA 3.40182 -37.5551
        FREQ 180e+6 1.0 0 0 0
        FREQ 190e+6 0.123 0.5 0 0
        COMPONENT 3.40200 -37.6
        GAUSSIAN 90 1.0 0.5
        FREQ 180e+6 0.5 0 0 0
        FREQ 170e+6 1.0 0 0.2 0
        ENDCOMPONENT
        COMPONENT 3.40200 -37.6
        SHAPELET2 70 1.5 0.75
        COEFF 0.0e+00   0.0e+00   5.0239939e-02
        COEFF 9.0e+00   0.0e+00  -8.7418484e-03
        FREQ 180e+6 0.5 0 0 0
        FREQ 170e+6 1.0 0 0.2 0
        ENDCOMPONENT
        ENDSOURCE
        SOURCE VLA_ForB 3.40166 -37.0551
        FREQ 180e+6 2.0 0 0 0
        ENDSOURCE
        "});

        let result = parse_source_list(&mut sl);
        assert!(result.is_ok(), "{:?}", result);
        let sl = result.unwrap();
        assert_eq!(sl.len(), 2);

        assert!(sl.contains_key("VLA_ForA"));
        let s = sl.get("VLA_ForA").unwrap();
        assert_eq!(s.components.len(), 3);
        let comp = &s.components[0];
        assert_abs_diff_eq!(comp.radec.ra, 3.40182 * DH2R);
        assert_abs_diff_eq!(comp.radec.dec, -37.5551_f64.to_radians());
        assert!(match comp.flux_type {
            FluxDensityType::List { .. } => true,
            _ => false,
        });
        let fds = match &comp.flux_type {
            FluxDensityType::List { fds } => fds,
            _ => unreachable!(),
        };
        assert_abs_diff_eq!(fds[0].freq, 180e6);
        assert_abs_diff_eq!(fds[0].i, 1.0);
        assert_abs_diff_eq!(fds[0].q, 0.0);
        assert_abs_diff_eq!(fds[1].i, 0.123);
        assert_abs_diff_eq!(fds[1].q, 0.5);

        let comp = &s.components[1];
        assert_abs_diff_eq!(comp.radec.ra, 3.402 * DH2R);
        assert_abs_diff_eq!(comp.radec.dec, -37.6_f64.to_radians());
        assert!(match comp.flux_type {
            FluxDensityType::List { .. } => true,
            _ => false,
        });
        let fds = match &comp.flux_type {
            FluxDensityType::List { fds } => fds,
            _ => unreachable!(),
        };
        // Note that 180 MHz isn't the first FREQ specified; the list has been
        // sorted.
        assert_abs_diff_eq!(fds[0].freq, 170e6);
        assert_abs_diff_eq!(fds[0].i, 1.0);
        assert_abs_diff_eq!(fds[0].q, 0.0);
        assert_abs_diff_eq!(fds[0].u, 0.2);
        assert_abs_diff_eq!(fds[1].freq, 180e6);
        assert_abs_diff_eq!(fds[1].i, 0.5);
        assert_abs_diff_eq!(fds[1].u, 0.0);

        assert!(sl.contains_key("VLA_ForB"));
        let s = sl.get("VLA_ForB").unwrap();
        assert_eq!(s.components.len(), 1);
        let comp = &s.components[0];
        assert_abs_diff_eq!(comp.radec.ra, 3.40166 * DH2R);
        assert_abs_diff_eq!(comp.radec.dec, -37.0551_f64.to_radians());
        assert!(match comp.flux_type {
            FluxDensityType::List { .. } => true,
            _ => false,
        });
        let fds = match &comp.flux_type {
            FluxDensityType::List { fds } => fds,
            _ => unreachable!(),
        };
        assert_abs_diff_eq!(fds[0].i, 2.0);
        assert_abs_diff_eq!(fds[0].q, 0.0);
    }

    #[test]
    fn parse_source_2_comps_freqs_swapped() {
        let mut sl = Cursor::new(indoc! {"
        SOURCE VLA_ForA 3.40182 -37.5551
        FREQ 180e+6 1.0 0 0 0
        FREQ 190e+6 0.123 0.5 0 0
        COMPONENT 3.40200 -37.6
        FREQ 180e+6 0.5 0 0 0
        FREQ 170e+6 1.0 0 0.2 0
        GAUSSIAN 90 1.0 0.5
        ENDCOMPONENT
        COMPONENT 3.40200 -37.6
        FREQ 180e+6 0.5 0 0 0
        FREQ 170e+6 1.0 0 0.2 0
        SHAPELET2 70 1.5 0.75
        COEFF 0.0e+00   0.0e+00   5.0239939e-02
        COEFF 9.0e+00   0.0e+00  -8.7418484e-03
        ENDCOMPONENT
        ENDSOURCE
        SOURCE VLA_ForB 3.40166 -37.0551
        FREQ 180e+6 2.0 0 0 0
        ENDSOURCE
        "});

        let result = parse_source_list(&mut sl);
        assert!(result.is_ok(), "{:?}", result);
        let sl = result.unwrap();
        assert_eq!(sl.len(), 2);
    }

    #[test]
    fn parse_source_3() {
        let mut sl = Cursor::new(indoc! {"
        SOURCE VLA_ForA 3.40182 -37.5551
        FREQ 180e+6 1.0 0 0 0
        # FREQ 190e+6 0.123 0.5 0 0
        ENDSOURCE
        SOURCE VLA_ForB 3.40166 -37.0551 # Fornax B >>> Fornax A
        GAUSSIAN 90 1.0 0.5
        # FREQ 180e+6 0.5 0 0 0
        FREQ 170e+6 1.0 0 0.2 0
        ENDSOURCE
        "});

        let result = parse_source_list(&mut sl);
        assert!(result.is_ok(), "{:?}", result);
        let sl = result.unwrap();
        assert_eq!(sl.len(), 2);
    }

    #[test]
    fn parse_source_4() {
        let mut sl = Cursor::new(indoc! {"
        SOURCE VLA_ForA 3.40182 -37.5551
        FREQ 180e+6 1.0 0 0 0
        GAUSSIAN 90 1.0 0.5
        COMPONENT 3.40166 -37.0551
        GAUSSIAN 90 1.0 0.5
        FREQ 170e+6 1.0 0 0.2 0
        ENDCOMPONENT
        ENDSOURCE
        "});

        let result = parse_source_list(&mut sl);
        assert!(result.is_ok(), "{:?}", result);
        let sl = result.unwrap();
        assert_eq!(sl.len(), 1);
        let comps = &sl.get("VLA_ForA").unwrap().components;
        assert_eq!(comps.len(), 2);
    }

    #[test]
    fn parse_flawed_source_commented() {
        let mut sl = Cursor::new(indoc! {"
        # SOURCE VLA_ForA 3.40182 -37.5551
        FREQ 190e+6 0.123 0.5 0 0
        ENDSOURCE
        "});
        let result = parse_source_list(&mut sl);
        assert!(&result.is_err(), "{:?}", &result);
    }

    #[test]
    fn parse_flawed_freq_commented() {
        let mut sl = Cursor::new(indoc! {"
        SOURCE VLA_ForA 3.40182 -37.5551
        # FREQ 190e+6 0.123 0.5 0 0
        ENDSOURCE
        "});
        let result = parse_source_list(&mut sl);
        assert!(&result.is_err(), "{:?}", &result);
    }

    #[test]
    fn parse_flawed_endsource_commented() {
        let mut sl = Cursor::new(indoc! {"
        SOURCE VLA_ForA 3.40182 -37.5551
        FREQ 190e+6 0.123 0.5 0 0
        # ENDSOURCE
        "});
        let result = parse_source_list(&mut sl);
        assert!(&result.is_err(), "{:?}", &result);
    }

    #[test]
    fn parse_flawed_endsource_indented() {
        let mut sl = Cursor::new(indoc! {"
        SOURCE VLA_ForA 3.40182 -37.5551
        FREQ 190e+6 0.123 0.5 0 0
         ENDSOURCE
        "});
        let result = parse_source_list(&mut sl);
        assert!(&result.is_err(), "{:?}", &result);
    }

    #[test]
    fn parse_flawed_shapelet() {
        // Because we ignore SHAPELET components, this source list has no sources.
        let mut sl = Cursor::new(indoc! {"
        SOURCE VLA_ForA 3.40182 -37.5551
        FREQ 170e6 235.71 0 0 0
        SHAPELET -77.99222 8.027761 4.820640
        COEFF 0.0e+00   0.0e+00   5.0239939e-02
        COEFF 0.0e+00   2.0e+00   2.0790306e-02
        ENDSOURCE
        "});
        let result = parse_source_list(&mut sl);
        assert!(&result.is_err(), "{:?}", &result);
    }

    #[test]
    fn parse_flawed_shapelet_comp() {
        // Because we ignore SHAPELET components, this source list has one
        // component.
        let mut sl = Cursor::new(indoc! {"
        SOURCE VLA_ForA 3.40182 -37.5551
        FREQ 170e6 235.71 0 0 0
        COMPONENT 3.40200 -37.6
        FREQ 170e6 200 0 0 0
        SHAPELET -77.99222 8.027761 4.820640
        COEFF 0.0e+00   0.0e+00   5.0239939e-02
        COEFF 0.0e+00   2.0e+00   2.0790306e-02
        ENDCOMPONENT
        ENDSOURCE
        "});
        let result = parse_source_list(&mut sl);
        assert!(&result.is_ok(), "{:?}", &result);
        let sl = result.unwrap();
        assert_eq!(sl.len(), 1);
        let comps = &sl.get("VLA_ForA").unwrap().components;
        assert_eq!(comps.len(), 1);
    }

    #[test]
    fn parse_flawed_no_shapelet2_coeffs() {
        // No shapelet COEFFs.
        let mut sl = Cursor::new(indoc! {"
        SOURCE VLA_ForA 3.40182 -37.5551
        FREQ 170e6 235.71 0 0 0
        SHAPELET2 -77.99222 8.027761 4.820640
        # COEFF 0.0e+00   0.0e+00   5.0239939e-02
        # COEFF 0.0e+00   2.0e+00   2.0790306e-02
        ENDSOURCE
        "});
        let result = parse_source_list(&mut sl);
        assert!(&result.is_err(), "{:?}", &result);
    }

    #[test]
    fn parse_flawed_no_shapelet2_coeffs_comp() {
        let mut sl = Cursor::new(indoc! {"
        SOURCE VLA_ForA 3.40182 -37.5551
        FREQ 170e6 235.71 0 0 0
        COMPONENT 3.40200 -37.6
        FREQ 170e6 200 0 0 0
        SHAPELET2 -77.99222 8.027761 4.820640
        # COEFF 0.0e+00   0.0e+00   5.0239939e-02
        # COEFF 0.0e+00   2.0e+00   2.0790306e-02
        ENDCOMPONENT
        ENDSOURCE
        "});
        let result = parse_source_list(&mut sl);
        assert!(&result.is_err(), "{:?}", &result);
    }

    #[test]
    fn parse_flawed_endcomponent_commented() {
        let mut sl = Cursor::new(indoc! {"
        SOURCE VLA_ForA 3.40182 -37.5551
        FREQ 190e+6 0.123 0.5 0 0
        COMPONENT 3.40200 -37.6
        FREQ 180e+6 0.5 0 0 0
        # ENDCOMPONENT
        ENDSOURCE
        "});
        let result = parse_source_list(&mut sl);
        assert!(&result.is_err(), "{:?}", &result);
    }

    #[test]
    fn parse_flawed_endcomponent_indented() {
        let mut sl = Cursor::new(indoc! {"
        SOURCE VLA_ForA 3.40182 -37.5551
        FREQ 190e+6 0.123 0.5 0 0
        COMPONENT 3.40200 -37.6
        FREQ 180e+6 0.5 0 0 0
        \tENDCOMPONENT
        ENDSOURCE
        "});
        let result = parse_source_list(&mut sl);
        assert!(&result.is_err(), "{:?}", &result);
    }

    #[test]
    fn invalid_flux_sum1() {
        let mut sl = Cursor::new(indoc! {"
        SOURCE VLA_ForA 3.40182 -37.5551
        FREQ 180e+6 0 1.0 0 0
        ENDSOURCE
        SOURCE VLA_ForB 3.40166 -37.0551
        FREQ 180e+6 0 -2.0 0 0
        ENDSOURCE
        "});

        let result = parse_source_list(&mut sl);
        assert!(result.is_err(), "{:?}", result);
        let err_message = format!("{}", result.unwrap_err());
        assert_eq!(
            err_message,
            "Source VLA_ForB: The sum of all Stokes Q flux densities was negative (-2)"
        );
    }

    #[test]
    fn invalid_flux_sum2() {
        let mut sl = Cursor::new(indoc! {"
        SOURCE VLA_ForA 3.40182 -37.5551
        FREQ 180e+6 0 1.0 0 0
        COMPONENT 3.40166 -37.0551
        FREQ 180e+6 0 -2.0 0 0
        ENDCOMPONENT
        ENDSOURCE
        "});

        let result = parse_source_list(&mut sl);
        assert!(result.is_err(), "{:?}", result);
        let err_message = format!("{}", result.unwrap_err());
        assert_eq!(
            err_message,
            "Source VLA_ForA: The sum of all Stokes Q flux densities was negative (-1)"
        );
    }
}
