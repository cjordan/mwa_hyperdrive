/* automatically generated by rust-bindgen 0.59.2 */

#[doc = " Common things needed to perform modelling. All pointers are to device"]
#[doc = " memory."]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Addresses {
    pub num_freqs: ::std::os::raw::c_int,
    pub num_vis: ::std::os::raw::c_int,
    pub num_tiles: ::std::os::raw::c_int,
    pub sbf_l: ::std::os::raw::c_int,
    pub sbf_n: ::std::os::raw::c_int,
    pub sbf_c: f64,
    pub sbf_dx: f64,
    pub d_freqs: *const f64,
    pub d_shapelet_basis_values: *const f64,
    pub num_unique_beam_freqs: ::std::os::raw::c_int,
    pub d_tile_map: *const ::std::os::raw::c_int,
    pub d_freq_map: *const ::std::os::raw::c_int,
    pub d_tile_index_to_unflagged_tile_index_map: *const ::std::os::raw::c_int,
    pub d_vis: *mut JonesF32,
}
#[test]
fn bindgen_test_layout_Addresses() {
    assert_eq!(
        ::std::mem::size_of::<Addresses>(),
        96usize,
        concat!("Size of: ", stringify!(Addresses))
    );
    assert_eq!(
        ::std::mem::align_of::<Addresses>(),
        8usize,
        concat!("Alignment of ", stringify!(Addresses))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Addresses>())).num_freqs as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(Addresses),
            "::",
            stringify!(num_freqs)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Addresses>())).num_vis as *const _ as usize },
        4usize,
        concat!(
            "Offset of field: ",
            stringify!(Addresses),
            "::",
            stringify!(num_vis)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Addresses>())).num_tiles as *const _ as usize },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(Addresses),
            "::",
            stringify!(num_tiles)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Addresses>())).sbf_l as *const _ as usize },
        12usize,
        concat!(
            "Offset of field: ",
            stringify!(Addresses),
            "::",
            stringify!(sbf_l)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Addresses>())).sbf_n as *const _ as usize },
        16usize,
        concat!(
            "Offset of field: ",
            stringify!(Addresses),
            "::",
            stringify!(sbf_n)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Addresses>())).sbf_c as *const _ as usize },
        24usize,
        concat!(
            "Offset of field: ",
            stringify!(Addresses),
            "::",
            stringify!(sbf_c)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Addresses>())).sbf_dx as *const _ as usize },
        32usize,
        concat!(
            "Offset of field: ",
            stringify!(Addresses),
            "::",
            stringify!(sbf_dx)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Addresses>())).d_freqs as *const _ as usize },
        40usize,
        concat!(
            "Offset of field: ",
            stringify!(Addresses),
            "::",
            stringify!(d_freqs)
        )
    );
    assert_eq!(
        unsafe {
            &(*(::std::ptr::null::<Addresses>())).d_shapelet_basis_values as *const _ as usize
        },
        48usize,
        concat!(
            "Offset of field: ",
            stringify!(Addresses),
            "::",
            stringify!(d_shapelet_basis_values)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Addresses>())).num_unique_beam_freqs as *const _ as usize },
        56usize,
        concat!(
            "Offset of field: ",
            stringify!(Addresses),
            "::",
            stringify!(num_unique_beam_freqs)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Addresses>())).d_tile_map as *const _ as usize },
        64usize,
        concat!(
            "Offset of field: ",
            stringify!(Addresses),
            "::",
            stringify!(d_tile_map)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Addresses>())).d_freq_map as *const _ as usize },
        72usize,
        concat!(
            "Offset of field: ",
            stringify!(Addresses),
            "::",
            stringify!(d_freq_map)
        )
    );
    assert_eq!(
        unsafe {
            &(*(::std::ptr::null::<Addresses>())).d_tile_index_to_unflagged_tile_index_map
                as *const _ as usize
        },
        80usize,
        concat!(
            "Offset of field: ",
            stringify!(Addresses),
            "::",
            stringify!(d_tile_index_to_unflagged_tile_index_map)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Addresses>())).d_vis as *const _ as usize },
        88usize,
        concat!(
            "Offset of field: ",
            stringify!(Addresses),
            "::",
            stringify!(d_vis)
        )
    );
}
