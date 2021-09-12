/* automatically generated by rust-bindgen 0.59.1 */

#[doc = " The return type of `allocate_init`. All pointers are to device memory, except"]
#[doc = " `host_vis`."]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Addresses {
    pub num_freqs: usize,
    pub num_vis: usize,
    pub sbf_l: usize,
    pub sbf_n: usize,
    pub sbf_c: f64,
    pub sbf_dx: f64,
    pub d_uvws: *mut UVW,
    pub d_freqs: *mut f64,
    pub d_shapelet_basis_values: *mut f64,
    pub d_vis: *mut JonesF32,
    pub host_vis: *mut JonesF32,
}
#[test]
fn bindgen_test_layout_Addresses() {
    assert_eq!(
        ::std::mem::size_of::<Addresses>(),
        88usize,
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
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(Addresses),
            "::",
            stringify!(num_vis)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Addresses>())).sbf_l as *const _ as usize },
        16usize,
        concat!(
            "Offset of field: ",
            stringify!(Addresses),
            "::",
            stringify!(sbf_l)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Addresses>())).sbf_n as *const _ as usize },
        24usize,
        concat!(
            "Offset of field: ",
            stringify!(Addresses),
            "::",
            stringify!(sbf_n)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Addresses>())).sbf_c as *const _ as usize },
        32usize,
        concat!(
            "Offset of field: ",
            stringify!(Addresses),
            "::",
            stringify!(sbf_c)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Addresses>())).sbf_dx as *const _ as usize },
        40usize,
        concat!(
            "Offset of field: ",
            stringify!(Addresses),
            "::",
            stringify!(sbf_dx)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Addresses>())).d_uvws as *const _ as usize },
        48usize,
        concat!(
            "Offset of field: ",
            stringify!(Addresses),
            "::",
            stringify!(d_uvws)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Addresses>())).d_freqs as *const _ as usize },
        56usize,
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
        64usize,
        concat!(
            "Offset of field: ",
            stringify!(Addresses),
            "::",
            stringify!(d_shapelet_basis_values)
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
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Addresses>())).host_vis as *const _ as usize },
        80usize,
        concat!(
            "Offset of field: ",
            stringify!(Addresses),
            "::",
            stringify!(host_vis)
        )
    );
}
extern "C" {
    #[doc = " Function to allocate necessary arrays (UVWs, frequencies and visibilities)"]
    #[doc = " for modelling on the device."]
    pub fn init_model(
        num_baselines: usize,
        num_freqs: usize,
        sbf_l: usize,
        sbf_n: usize,
        sbf_c: f64,
        sbf_dx: f64,
        uvws: *const UVW,
        freqs: *const f64,
        shapelet_basis_values: *const f64,
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
    pub fn clear_vis(a: *const Addresses);
}
extern "C" {
    #[doc = " Deallocate necessary arrays (UVWs, frequencies and visibilities) on the"]
    #[doc = " device."]
    pub fn destroy(addresses: *const Addresses);
}