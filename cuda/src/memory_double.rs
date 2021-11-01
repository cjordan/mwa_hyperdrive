/* automatically generated by rust-bindgen 0.59.1 */

#[doc = " The return type of `allocate_init`. All pointers are to device memory."]
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
    pub d_freqs: *mut f64,
    pub d_shapelet_basis_values: *mut f64,
    pub num_unique_beam_freqs: ::std::os::raw::c_int,
    pub d_beam_jones_map: *const u64,
    pub d_vis: *mut JonesF32,
}
#[test]
fn bindgen_test_layout_Addresses() {
    assert_eq!(
        ::std::mem::size_of::<Addresses>(),
        80usize,
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
        unsafe { &(*(::std::ptr::null::<Addresses>())).d_beam_jones_map as *const _ as usize },
        64usize,
        concat!(
            "Offset of field: ",
            stringify!(Addresses),
            "::",
            stringify!(d_beam_jones_map)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Addresses>())).d_vis as *const _ as usize },
        72usize,
        concat!(
            "Offset of field: ",
            stringify!(Addresses),
            "::",
            stringify!(d_vis)
        )
    );
}
extern "C" {
    #[doc = " Function to allocate necessary arrays (UVWs, frequencies and visibilities)"]
    #[doc = " for modelling on the device."]
    pub fn init_model(
        num_baselines: ::std::os::raw::c_int,
        num_freqs: ::std::os::raw::c_int,
        num_tiles: ::std::os::raw::c_int,
        sbf_l: ::std::os::raw::c_int,
        sbf_n: ::std::os::raw::c_int,
        sbf_c: f64,
        sbf_dx: f64,
        uvws: *mut UVW,
        freqs: *mut f64,
        shapelet_basis_values: *mut f64,
        d_fee_coeffs: *mut ::std::os::raw::c_void,
        num_fee_beam_coeffs: ::std::os::raw::c_int,
        num_unique_fee_tiles: ::std::os::raw::c_int,
        num_unique_fee_freqs: ::std::os::raw::c_int,
        d_beam_jones_map: *mut u64,
        d_beam_norm_jones: *mut ::std::os::raw::c_void,
        vis: *mut JonesF32,
    ) -> Addresses;
}
extern "C" {
    #[doc = " Copy the device visibilities to the host. It is assumed that this operation"]
    #[doc = " always succeeds so not status int is returned."]
    pub fn copy_vis(addresses: *const Addresses);
}
extern "C" {
    #[doc = " Set all of the visibilities to zero."]
    pub fn clear_vis(a: *mut Addresses);
}
extern "C" {
    #[doc = " Deallocate necessary arrays (UVWs, frequencies and visibilities) on the"]
    #[doc = " device."]
    pub fn destroy(addresses: *mut Addresses);
}
