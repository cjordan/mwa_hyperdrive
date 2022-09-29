// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Bad name.

use std::{
    fs::File,
    io::{BufReader, Cursor, Read},
};

use approx::assert_abs_diff_eq;
use marlu::RADec;
use tempfile::NamedTempFile;
use vec1::{vec1, Vec1};

use super::*;
use crate::constants::DEFAULT_SPEC_INDEX;

fn test_two_sources_lists_are_the_same(sl1: &SourceList, sl2: &SourceList) {
    assert_eq!(sl1.len(), sl2.len());
    for ((sl1_name, s1), (sl2_name, s2)) in sl1.iter().zip(sl2.iter()) {
        assert_eq!(sl1_name, sl2_name);
        assert_eq!(s1.components.len(), s2.components.len());
        for (s1_comp, s2_comp) in s1.components.iter().zip(s2.components.iter()) {
            // Tolerances are a little looser here because AO source lists
            // use sexagesimal, so the floating-point errors compound
            // significantly when converting.
            assert_abs_diff_eq!(s1_comp.radec.ra, s2_comp.radec.ra, epsilon = 1e-8);
            assert_abs_diff_eq!(s1_comp.radec.dec, s2_comp.radec.dec, epsilon = 1e-8);
            match &s1_comp.comp_type {
                ComponentType::Point => {
                    assert!(matches!(s2_comp.comp_type, ComponentType::Point))
                }

                ComponentType::Gaussian { maj, min, pa } => {
                    assert!(matches!(s2_comp.comp_type, ComponentType::Gaussian { .. }));
                    let s1_maj = *maj;
                    let s1_min = *min;
                    let s1_pa = *pa;
                    match s2_comp.comp_type {
                        ComponentType::Gaussian { maj, min, pa } => {
                            assert_abs_diff_eq!(s1_maj, maj, epsilon = 1e-10);
                            assert_abs_diff_eq!(s1_min, min, epsilon = 1e-10);
                            assert_abs_diff_eq!(s1_pa, pa, epsilon = 1e-10);
                        }
                        _ => unreachable!(),
                    }
                }

                ComponentType::Shapelet {
                    maj,
                    min,
                    pa,
                    coeffs,
                } => {
                    assert!(matches!(s2_comp.comp_type, ComponentType::Shapelet { .. }));
                    let s1_maj = *maj;
                    let s1_min = *min;
                    let s1_pa = *pa;
                    let s1_coeffs = coeffs;
                    match &s2_comp.comp_type {
                        ComponentType::Shapelet {
                            maj,
                            min,
                            pa,
                            coeffs,
                        } => {
                            assert_abs_diff_eq!(s1_maj, maj, epsilon = 1e-10);
                            assert_abs_diff_eq!(s1_min, min, epsilon = 1e-10);
                            assert_abs_diff_eq!(s1_pa, pa, epsilon = 1e-10);
                            for (s1_coeff, s2_coeff) in s1_coeffs.iter().zip(coeffs.iter()) {
                                assert_eq!(s1_coeff.n1, s2_coeff.n1);
                                assert_eq!(s1_coeff.n2, s2_coeff.n2);
                                assert_abs_diff_eq!(
                                    s1_coeff.value,
                                    s2_coeff.value,
                                    epsilon = 1e-10
                                );
                            }
                        }
                        _ => unreachable!(),
                    }
                }
            }

            match &s1_comp.flux_type {
                FluxDensityType::List { fds } => {
                    assert!(
                        matches!(s2_comp.flux_type, FluxDensityType::List { .. }),
                        "{:?} {:?}",
                        s1_comp,
                        s2_comp
                    );
                    let s1_fds = fds;
                    match &s2_comp.flux_type {
                        FluxDensityType::List { fds } => {
                            assert_eq!(s1_fds.len(), fds.len());
                            for (s1_fd, s2_fd) in s1_fds.iter().zip(fds.iter()) {
                                assert_abs_diff_eq!(s1_fd.freq, s2_fd.freq, epsilon = 1e-10);
                                assert_abs_diff_eq!(s1_fd.i, s2_fd.i, epsilon = 1e-10);
                                assert_abs_diff_eq!(s1_fd.q, s2_fd.q, epsilon = 1e-10);
                                assert_abs_diff_eq!(s1_fd.u, s2_fd.u, epsilon = 1e-10);
                                assert_abs_diff_eq!(s1_fd.v, s2_fd.v, epsilon = 1e-10);
                            }
                        }
                        _ => unreachable!(),
                    }
                }

                FluxDensityType::PowerLaw { .. } => {
                    assert!(matches!(
                        s2_comp.flux_type,
                        FluxDensityType::PowerLaw { .. }
                    ));
                    match s2_comp.flux_type {
                        FluxDensityType::PowerLaw { .. } => {
                            // The parameters of the power law may not
                            // match, but the estimated flux densities
                            // should.
                            let s1_fd = s1_comp.flux_type.estimate_at_freq(150e6);
                            let s2_fd = s2_comp.flux_type.estimate_at_freq(150e6);
                            assert_abs_diff_eq!(s1_fd.freq, s2_fd.freq, epsilon = 1e-10);
                            assert_abs_diff_eq!(s1_fd.i, s2_fd.i, epsilon = 1e-10);
                            assert_abs_diff_eq!(s1_fd.q, s2_fd.q, epsilon = 1e-10);
                            assert_abs_diff_eq!(s1_fd.u, s2_fd.u, epsilon = 1e-10);
                            assert_abs_diff_eq!(s1_fd.v, s2_fd.v, epsilon = 1e-10);
                            let s1_fd = s1_comp.flux_type.estimate_at_freq(250e6);
                            let s2_fd = s2_comp.flux_type.estimate_at_freq(250e6);
                            assert_abs_diff_eq!(s1_fd.freq, s2_fd.freq, epsilon = 1e-10);
                            assert_abs_diff_eq!(s1_fd.i, s2_fd.i, epsilon = 1e-10);
                            assert_abs_diff_eq!(s1_fd.q, s2_fd.q, epsilon = 1e-10);
                            assert_abs_diff_eq!(s1_fd.u, s2_fd.u, epsilon = 1e-10);
                            assert_abs_diff_eq!(s1_fd.v, s2_fd.v, epsilon = 1e-10);
                        }
                        _ => unreachable!(),
                    }
                }

                FluxDensityType::CurvedPowerLaw { .. } => {
                    assert!(matches!(
                        s2_comp.flux_type,
                        FluxDensityType::PowerLaw { .. }
                    ));
                    match s2_comp.flux_type {
                        FluxDensityType::CurvedPowerLaw { .. } => {
                            // The parameters of the curved power law may
                            // not match, but the estimated flux densities
                            // should.
                            let s1_fd = s1_comp.flux_type.estimate_at_freq(150e6);
                            let s2_fd = s2_comp.flux_type.estimate_at_freq(150e6);
                            assert_abs_diff_eq!(s1_fd.freq, s2_fd.freq, epsilon = 1e-10);
                            assert_abs_diff_eq!(s1_fd.i, s2_fd.i, epsilon = 1e-10);
                            assert_abs_diff_eq!(s1_fd.q, s2_fd.q, epsilon = 1e-10);
                            assert_abs_diff_eq!(s1_fd.u, s2_fd.u, epsilon = 1e-10);
                            assert_abs_diff_eq!(s1_fd.v, s2_fd.v, epsilon = 1e-10);
                            let s1_fd = s1_comp.flux_type.estimate_at_freq(250e6);
                            let s2_fd = s2_comp.flux_type.estimate_at_freq(250e6);
                            assert_abs_diff_eq!(s1_fd.freq, s2_fd.freq, epsilon = 1e-10);
                            assert_abs_diff_eq!(s1_fd.i, s2_fd.i, epsilon = 1e-10);
                            assert_abs_diff_eq!(s1_fd.q, s2_fd.q, epsilon = 1e-10);
                            assert_abs_diff_eq!(s1_fd.u, s2_fd.u, epsilon = 1e-10);
                            assert_abs_diff_eq!(s1_fd.v, s2_fd.v, epsilon = 1e-10);
                        }
                        _ => unreachable!(),
                    }
                }
            }
        }
    }
}

#[test]
fn hyperdrive_conversion_works() {
    let mut sl = SourceList::new();
    sl.insert(
        "gaussian".to_string(),
        Source {
            components: vec1![SourceComponent {
                radec: RADec::new_degrees(61.0, -28.0),
                comp_type: ComponentType::Gaussian {
                    maj: 1.0,
                    min: 0.5,
                    pa: 90.0,
                },
                flux_type: FluxDensityType::PowerLaw {
                    si: DEFAULT_SPEC_INDEX,
                    fd: FluxDensity {
                        freq: 190e6,
                        i: 11.0,
                        q: 1.0,
                        u: 2.0,
                        v: 3.0,
                    },
                },
            }],
        },
    );
    sl.insert(
        "point".to_string(),
        Source {
            components: vec1![SourceComponent {
                radec: RADec::new_degrees(60.0, -27.0),
                comp_type: ComponentType::Point,
                flux_type: FluxDensityType::PowerLaw {
                    si: DEFAULT_SPEC_INDEX,
                    fd: FluxDensity {
                        freq: 180e6,
                        i: 10.0,
                        q: 1.0,
                        u: 2.0,
                        v: 3.0,
                    },
                },
            }],
        },
    );
    sl.insert(
        "shapelet".to_string(),
        Source {
            components: vec1![SourceComponent {
                radec: RADec::new_degrees(59.0, -26.0),
                comp_type: ComponentType::Shapelet {
                    maj: 1.0,
                    min: 0.5,
                    pa: 90.0,
                    coeffs: vec![ShapeletCoeff {
                        n1: 0,
                        n2: 0,
                        value: 1.0,
                    }],
                },
                flux_type: FluxDensityType::PowerLaw {
                    si: DEFAULT_SPEC_INDEX,
                    fd: FluxDensity {
                        freq: 170e6,
                        i: 9.0,
                        q: 1.0,
                        u: 2.0,
                        v: 3.0,
                    },
                },
            }],
        },
    );

    // Write this source list as hyperdrive style into a buffer.
    let mut buf = Cursor::new(vec![]);
    hyperdrive::source_list_to_yaml(&mut buf, &sl, None).unwrap();
    // Read it back in, and test that things are sensible.
    buf.set_position(0);
    let new_sl = hyperdrive::source_list_from_yaml(&mut buf).unwrap();
    test_two_sources_lists_are_the_same(&sl, &new_sl);
}

#[test]
fn rts_conversion_works() {
    let (hyperdrive_sl, _) = read::read_source_list_file(
        "test_files/srclist_1099334672_100.yaml",
        Some(SourceListType::Hyperdrive),
    )
    .unwrap();
    let mut buf = Cursor::new(vec![]);
    rts::write_source_list(&mut buf, &hyperdrive_sl, None).unwrap();
    buf.set_position(0);
    let rts_sl = rts::parse_source_list(&mut buf).unwrap();
    test_two_sources_lists_are_the_same(&hyperdrive_sl, &rts_sl);
}

#[test]
fn ao_conversion_works() {
    let (hyperdrive_sl, _) = read::read_source_list_file(
        "test_files/srclist_1099334672_100.yaml",
        Some(SourceListType::Hyperdrive),
    )
    .unwrap();
    let mut buf = Cursor::new(vec![]);

    // AO source lists don't handle shapelets. Prune those from the
    // hyperdrive source list before continuing.
    let mut new_hyperdrive_sl = SourceList::new();
    for (name, src) in hyperdrive_sl.into_iter() {
        let comps: Vec<_> = src
            .components
            .into_iter()
            .filter(|comp| !matches!(comp.comp_type, ComponentType::Shapelet { .. }))
            .collect();
        if !comps.is_empty() {
            new_hyperdrive_sl.insert(
                name,
                Source {
                    components: Vec1::try_from_vec(comps).unwrap(),
                },
            );
        }
    }

    ao::write_source_list(&mut buf, &new_hyperdrive_sl, None).unwrap();
    buf.set_position(0);
    let ao_sl = ao::parse_source_list(&mut buf).unwrap();
    test_two_sources_lists_are_the_same(&new_hyperdrive_sl, &ao_sl);
}

#[test]
fn woden_conversion_works() {
    let (mut hyperdrive_sl, _) = read::read_source_list_file(
        "test_files/srclist_1099334672_100.yaml",
        Some(SourceListType::Hyperdrive),
    )
    .unwrap();

    let mut buf = Cursor::new(vec![]);
    woden::write_source_list(&mut buf, &hyperdrive_sl, None).unwrap();
    buf.set_position(0);
    let woden_sl = woden::parse_source_list(&mut buf).unwrap();

    // WODEN only allows one flux density per component. Verify that things
    // are different.
    let mut flux_density_types_are_different = false;
    for (hyp_comp, woden_comp) in hyperdrive_sl
        .iter()
        .flat_map(|(_, s)| &s.components)
        .zip(woden_sl.iter().flat_map(|(_, s)| &s.components))
    {
        if std::mem::discriminant(&hyp_comp.flux_type)
            != std::mem::discriminant(&woden_comp.flux_type)
        {
            flux_density_types_are_different = true;
            break;
        }
    }
    assert!(flux_density_types_are_different);

    // Now alter the hyperdrive source list to match WODEN.
    for comp in hyperdrive_sl
        .iter_mut()
        .flat_map(|(_, s)| &mut s.components)
    {
        comp.flux_type = match &comp.flux_type {
            FluxDensityType::List { fds } => FluxDensityType::PowerLaw {
                fd: fds[0].clone(),
                si: DEFAULT_SPEC_INDEX,
            },
            FluxDensityType::PowerLaw { fd, si } => FluxDensityType::PowerLaw {
                si: *si,
                fd: fd.clone(),
            },
            FluxDensityType::CurvedPowerLaw { .. } => panic!(
                "Source list has a curved power law, but it shouldn't, and WODEN can't handle it."
            ),
        };
    }
    test_two_sources_lists_are_the_same(&hyperdrive_sl, &woden_sl);
}

#[test]
fn hyp_has_no_unsupported_things() {
    let (sl, _) = read::read_source_list_file(
        "test_files/srclist_all_kinds.yaml",
        Some(SourceListType::Hyperdrive),
    )
    .unwrap();

    let mut buf = Cursor::new(vec![]);
    let result = hyperdrive::source_list_to_yaml(&mut buf, &sl, None);
    assert!(result.is_ok());
    result.unwrap();

    buf.set_position(0);
    let sl = hyperdrive::source_list_from_yaml(&mut buf).unwrap();
    // 9 input sources, 9 output sources.
    assert_eq!(sl.len(), 9);
}

#[test]
fn rts_fails_on_unsupported_things() {
    let (sl, _) = read::read_source_list_file(
        "test_files/srclist_all_kinds.yaml",
        Some(SourceListType::Hyperdrive),
    )
    .unwrap();

    let mut buf = Cursor::new(vec![]);
    let result = rts::write_source_list(&mut buf, &sl, None);
    assert!(result.is_ok());
    result.unwrap();

    buf.set_position(0);
    let sl = rts::parse_source_list(&mut buf).unwrap();
    // Even though the RTS doesn't support power laws, hyperdrive kindly writes
    // them out as something it understands. The same is not done for curved
    // power laws though.
    assert_eq!(sl.len(), 6);

    for (_, src) in sl {
        assert!(!src
            .components
            .iter()
            .any(|comp| matches!(comp.flux_type, FluxDensityType::CurvedPowerLaw { .. })));
    }
}

#[test]
fn ao_fails_on_unsupported_things() {
    let (sl, _) = read::read_source_list_file(
        "test_files/srclist_all_kinds.yaml",
        Some(SourceListType::Hyperdrive),
    )
    .unwrap();

    let mut buf = Cursor::new(vec![]);
    let result = ao::write_source_list(&mut buf, &sl, None);
    assert!(result.is_ok());
    result.unwrap();

    buf.set_position(0);
    let sl = ao::parse_source_list(&mut buf).unwrap();
    // AO source lists don't support shapelets or curved power laws.
    assert_eq!(sl.len(), 4);
    for (_, src) in sl {
        assert!(!src
            .components
            .iter()
            .any(|comp| matches!(comp.comp_type, ComponentType::Shapelet { .. })));
        assert!(!src
            .components
            .iter()
            .any(|comp| matches!(comp.flux_type, FluxDensityType::CurvedPowerLaw { .. })));
    }
}

#[test]
fn woden_fails_on_unsupported_things() {
    let (sl, _) = read::read_source_list_file(
        "test_files/srclist_all_kinds.yaml",
        Some(SourceListType::Hyperdrive),
    )
    .unwrap();

    let mut buf = Cursor::new(vec![]);
    let result = woden::write_source_list(&mut buf, &sl, None);
    assert!(result.is_ok());
    result.unwrap();

    buf.set_position(0);
    let sl = woden::parse_source_list(&mut buf).unwrap();
    // WODEN source lists don't support curved power laws.
    assert_eq!(sl.len(), 6);
    for (_, src) in sl {
        assert!(!src
            .components
            .iter()
            .any(|comp| matches!(comp.flux_type, FluxDensityType::CurvedPowerLaw { .. })));
    }
}

#[test]
fn rts_write_throws_away_unsupported_things() {
    let (orig_sl, _) = read::read_source_list_file(
        "test_files/srclist_all_kinds.yaml",
        Some(SourceListType::Hyperdrive),
    )
    .unwrap();

    // Make a new source list containing only lists and curved power laws. The
    // RTS writer should only keep the lists.
    let mut sl = SourceList::new();
    for i in 0..5 {
        sl.insert(format!("point-list-{i}"), orig_sl["point-list"].clone());
        sl.insert(
            format!("point-curved-{i}"),
            orig_sl["point-curved-power-law"].clone(),
        );
        sl.insert(
            format!("gaussian-list-{i}"),
            orig_sl["gaussian-list"].clone(),
        );
        sl.insert(
            format!("gaussian-curved-{i}"),
            orig_sl["gaussian-curved-power-law"].clone(),
        );
        sl.insert(
            format!("shapelet-list-{i}"),
            orig_sl["shapelet-list"].clone(),
        );
        sl.insert(
            format!("shapelet-curved-{i}"),
            orig_sl["shapelet-curved-power-law"].clone(),
        );
    }

    let mut buf = Cursor::new(vec![]);
    rts::write_source_list(&mut buf, &sl, None).unwrap();
    buf.set_position(0);
    let rts_sl = rts::parse_source_list(&mut buf).unwrap();

    // There were 30 sources fed to the write, but only 15 exist.
    assert_eq!(rts_sl.len(), 15);
    // No curved power laws.
    for (_, src) in rts_sl {
        assert!(!src
            .components
            .iter()
            .any(|comp| matches!(comp.flux_type, FluxDensityType::CurvedPowerLaw { .. })));
    }
}

#[test]
fn ao_write_throws_away_unsupported_things() {
    let (orig_sl, _) = read::read_source_list_file(
        "test_files/srclist_all_kinds.yaml",
        Some(SourceListType::Hyperdrive),
    )
    .unwrap();

    // Make a new source list containing only lists and curved power laws. The
    // AO writer should only keep the lists and throw away shapelets.
    let mut sl = SourceList::new();
    for i in 0..5 {
        sl.insert(format!("point-list-{i}"), orig_sl["point-list"].clone());
        sl.insert(
            format!("point-curved-{i}"),
            orig_sl["point-curved-power-law"].clone(),
        );
        sl.insert(
            format!("gaussian-list-{i}"),
            orig_sl["gaussian-list"].clone(),
        );
        sl.insert(
            format!("gaussian-curved-{i}"),
            orig_sl["gaussian-curved-power-law"].clone(),
        );
        sl.insert(
            format!("shapelet-list-{i}"),
            orig_sl["shapelet-list"].clone(),
        );
        sl.insert(
            format!("shapelet-curved-{i}"),
            orig_sl["shapelet-curved-power-law"].clone(),
        );
    }

    let mut buf = Cursor::new(vec![]);
    ao::write_source_list(&mut buf, &sl, None).unwrap();
    buf.set_position(0);
    let ao_sl = ao::parse_source_list(&mut buf).unwrap();

    // There were 30 sources fed to the write, but only 10 exist.
    assert_eq!(ao_sl.len(), 10);
    // No curved power laws or shapelets.
    for (_, src) in ao_sl {
        assert!(!src
            .components
            .iter()
            .any(|comp| matches!(comp.flux_type, FluxDensityType::CurvedPowerLaw { .. })));
        assert!(!src
            .components
            .iter()
            .any(|comp| matches!(comp.comp_type, ComponentType::Shapelet { .. })));
    }
}

#[test]
fn woden_write_throws_away_unsupported_things() {
    let (orig_sl, _) = read::read_source_list_file(
        "test_files/srclist_all_kinds.yaml",
        Some(SourceListType::Hyperdrive),
    )
    .unwrap();

    // Make a new source list containing only power laws and curved power laws.
    // The WODEN writer should only keep the power laws.
    let mut sl = SourceList::new();
    for i in 0..5 {
        sl.insert(
            format!("point-power-law-{i}"),
            orig_sl["point-power-law"].clone(),
        );
        sl.insert(
            format!("point-curved-{i}"),
            orig_sl["point-curved-power-law"].clone(),
        );
        sl.insert(
            format!("gaussian-power-law-{i}"),
            orig_sl["gaussian-power-law"].clone(),
        );
        sl.insert(
            format!("gaussian-curved-{i}"),
            orig_sl["gaussian-curved-power-law"].clone(),
        );
        sl.insert(
            format!("shapelet-power-law-{i}"),
            orig_sl["shapelet-power-law"].clone(),
        );
        sl.insert(
            format!("shapelet-curved-{i}"),
            orig_sl["shapelet-curved-power-law"].clone(),
        );
    }

    let mut buf = Cursor::new(vec![]);
    woden::write_source_list(&mut buf, &sl, None).unwrap();
    buf.set_position(0);
    let woden_sl = woden::parse_source_list(&mut buf).unwrap();

    // There were 30 sources fed to the write, but only 15 exist.
    assert_eq!(woden_sl.len(), 15);
    // No curved power laws.
    for (_, src) in woden_sl {
        assert!(!src
            .components
            .iter()
            .any(|comp| matches!(comp.flux_type, FluxDensityType::CurvedPowerLaw { .. })));
    }
}

/// The return value of this function is the sourcelist that should match
/// what is in the examples.
fn get_example_sl() -> SourceList {
    let source1 = Source {
        components: vec1![SourceComponent {
            radec: RADec::new_degrees(10.0, -27.0),
            comp_type: ComponentType::Point,
            flux_type: FluxDensityType::List {
                fds: vec1![
                    FluxDensity {
                        freq: 150e6,
                        i: 10.0,
                        q: 0.0,
                        u: 0.0,
                        v: 0.0,
                    },
                    FluxDensity {
                        freq: 170e6,
                        i: 5.0,
                        q: 1.0,
                        u: 2.0,
                        v: 3.0,
                    },
                ],
            },
        }],
    };
    let source2 = Source {
        components: vec1![
            SourceComponent {
                radec: RADec::new_degrees(0.0, -35.0),
                comp_type: ComponentType::Gaussian {
                    maj: 20.0_f64.to_radians() / 3600.0,
                    min: 10.0_f64.to_radians() / 3600.0,
                    pa: 75.0_f64.to_radians(),
                },
                flux_type: FluxDensityType::PowerLaw {
                    si: -0.8,
                    fd: FluxDensity {
                        freq: 170e6,
                        i: 5.0,
                        q: 1.0,
                        u: 2.0,
                        v: 3.0,
                    },
                },
            },
            SourceComponent {
                radec: RADec::new_degrees(155.0, -10.0),
                comp_type: ComponentType::Shapelet {
                    maj: 20.0_f64.to_radians() / 3600.0,
                    min: 10.0_f64.to_radians() / 3600.0,
                    pa: 75.0_f64.to_radians(),
                    coeffs: vec![ShapeletCoeff {
                        n1: 0,
                        n2: 1,
                        value: 0.5,
                    }],
                },
                flux_type: FluxDensityType::CurvedPowerLaw {
                    si: -0.6,
                    fd: FluxDensity {
                        freq: 150e6,
                        i: 50.0,
                        q: 0.5,
                        u: 0.1,
                        v: 0.0,
                    },
                    q: 0.2,
                },
            },
        ],
    };

    let mut sl = SourceList::new();
    sl.insert("super_sweet_source1".to_string(), source1);
    sl.insert("super_sweet_source2".to_string(), source2);
    sl
}

#[test]
fn read_yaml_file() {
    let f = File::open("test_files/hyperdrive_srclist.yaml");
    assert!(f.is_ok(), "{}", f.unwrap_err());
    let mut f = BufReader::new(f.unwrap());

    let result = hyperdrive::source_list_from_yaml(&mut f);
    assert!(result.is_ok(), "{}", result.unwrap_err());
}

#[test]
fn read_json_file() {
    let f = File::open("test_files/hyperdrive_srclist.json");
    assert!(f.is_ok(), "{}", f.unwrap_err());
    let mut f = BufReader::new(f.unwrap());

    let result = hyperdrive::source_list_from_json(&mut f);
    assert!(result.is_ok(), "{}", result.unwrap_err());
}

#[test]
fn write_yaml_file() {
    let mut temp = NamedTempFile::new().unwrap();
    let sl = get_example_sl();
    let result = hyperdrive::source_list_to_yaml(&mut temp, &sl, None);
    assert!(result.is_ok(), "{}", result.unwrap_err());

    // Compare file contents. Do they match?
    let mut example = String::new();
    let mut just_written = String::new();

    let f = File::open("test_files/hyperdrive_srclist.yaml");
    assert!(f.is_ok(), "{}", f.unwrap_err());
    f.unwrap().read_to_string(&mut example).unwrap();

    let f = File::open(temp.path());
    assert!(f.is_ok(), "{}", f.unwrap_err());
    f.unwrap().read_to_string(&mut just_written).unwrap();

    // Use trim to ignore any leading or trailing whitespace; we don't care
    // about that when comparing contents.
    assert_eq!(example.trim(), just_written.trim());
}

#[test]
fn write_json_file() {
    let mut temp = NamedTempFile::new().unwrap();
    let sl = get_example_sl();
    let result = hyperdrive::source_list_to_json(&mut temp, &sl, None);
    assert!(result.is_ok(), "{}", result.unwrap_err());

    // Compare file contents. Do they match?
    let mut example = String::new();
    let mut just_written = String::new();
    {
        let f = File::open("test_files/hyperdrive_srclist.json");
        assert!(f.is_ok(), "{}", f.unwrap_err());
        f.unwrap().read_to_string(&mut example).unwrap();

        let f = File::open(temp.path());
        assert!(f.is_ok(), "{}", f.unwrap_err());
        f.unwrap().read_to_string(&mut just_written).unwrap();
        // Apparently there's a new line missing.
        just_written.push('\n');
    }
    assert_eq!(example, just_written);
}