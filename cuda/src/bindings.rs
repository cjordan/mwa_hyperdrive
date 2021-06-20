/* automatically generated by rust-bindgen 0.58.1 */

#[doc = " A struct containing direction-cosine coordinates for a single source"]
#[doc = " component."]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct LMN_c {
    #[doc = " l-coordinate [dimensionless]"]
    pub l: f32,
    #[doc = " m-coordinate [dimensionless]"]
    pub m: f32,
    #[doc = " n-coordinate [dimensionless]"]
    pub n: f32,
}
#[test]
fn bindgen_test_layout_LMN_c() {
    assert_eq!(
        ::std::mem::size_of::<LMN_c>(),
        12usize,
        concat!("Size of: ", stringify!(LMN_c))
    );
    assert_eq!(
        ::std::mem::align_of::<LMN_c>(),
        4usize,
        concat!("Alignment of ", stringify!(LMN_c))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<LMN_c>())).l as *const _ as usize },
        0usize,
        concat!("Offset of field: ", stringify!(LMN_c), "::", stringify!(l))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<LMN_c>())).m as *const _ as usize },
        4usize,
        concat!("Offset of field: ", stringify!(LMN_c), "::", stringify!(m))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<LMN_c>())).n as *const _ as usize },
        8usize,
        concat!("Offset of field: ", stringify!(LMN_c), "::", stringify!(n))
    );
}
#[doc = " A struct containing UVW coordinates for a single baseline."]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct UVW_c {
    #[doc = " u-coordinate [dimensionless]"]
    pub u: f32,
    #[doc = " v-coordinate [dimensionless]"]
    pub v: f32,
    #[doc = " w-coordinate [dimensionless]"]
    pub w: f32,
}
#[test]
fn bindgen_test_layout_UVW_c() {
    assert_eq!(
        ::std::mem::size_of::<UVW_c>(),
        12usize,
        concat!("Size of: ", stringify!(UVW_c))
    );
    assert_eq!(
        ::std::mem::align_of::<UVW_c>(),
        4usize,
        concat!("Alignment of ", stringify!(UVW_c))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<UVW_c>())).u as *const _ as usize },
        0usize,
        concat!("Offset of field: ", stringify!(UVW_c), "::", stringify!(u))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<UVW_c>())).v as *const _ as usize },
        4usize,
        concat!("Offset of field: ", stringify!(UVW_c), "::", stringify!(v))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<UVW_c>())).w as *const _ as usize },
        8usize,
        concat!("Offset of field: ", stringify!(UVW_c), "::", stringify!(w))
    );
}
#[doc = " A struct containing metadata on the observation."]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Context_c {
    #[doc = " The observation's frequency resolution [Hz]"]
    pub fine_channel_width: f64,
    #[doc = " The base frequency of the observation [Hz]"]
    pub base_freq: f64,
    #[doc = " The number of frequency channels (num. freq. bands * num. fine channels)"]
    #[doc = " present."]
    pub n_channels: ::std::os::raw::c_uint,
    #[doc = " The number of baselines present."]
    pub n_baselines: ::std::os::raw::c_uint,
}
#[test]
fn bindgen_test_layout_Context_c() {
    assert_eq!(
        ::std::mem::size_of::<Context_c>(),
        24usize,
        concat!("Size of: ", stringify!(Context_c))
    );
    assert_eq!(
        ::std::mem::align_of::<Context_c>(),
        8usize,
        concat!("Alignment of ", stringify!(Context_c))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Context_c>())).fine_channel_width as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(Context_c),
            "::",
            stringify!(fine_channel_width)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Context_c>())).base_freq as *const _ as usize },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(Context_c),
            "::",
            stringify!(base_freq)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Context_c>())).n_channels as *const _ as usize },
        16usize,
        concat!(
            "Offset of field: ",
            stringify!(Context_c),
            "::",
            stringify!(n_channels)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Context_c>())).n_baselines as *const _ as usize },
        20usize,
        concat!(
            "Offset of field: ",
            stringify!(Context_c),
            "::",
            stringify!(n_baselines)
        )
    );
}
#[doc = " A struct representing a source's components. Assumes that there is one"]
#[doc = " (l,m,n) per component, and `n_channels` Stokes I flux densities per"]
#[doc = " component."]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Source_c {
    #[doc = " The number of point source components."]
    pub n_points: ::std::os::raw::c_uint,
    #[doc = " LMN coordinates for each point-source component [dimensionless]"]
    pub point_lmn: *const LMN_c,
    #[doc = " The point-source flux densities [Jy]. The length of this array should be"]
    #[doc = " `n_points` * `n_channels`."]
    pub point_fd: *const f32,
    #[doc = " The number of frequency channels (num. freq. bands * num. fine channels)"]
    #[doc = " present."]
    pub n_channels: ::std::os::raw::c_uint,
}
#[test]
fn bindgen_test_layout_Source_c() {
    assert_eq!(
        ::std::mem::size_of::<Source_c>(),
        32usize,
        concat!("Size of: ", stringify!(Source_c))
    );
    assert_eq!(
        ::std::mem::align_of::<Source_c>(),
        8usize,
        concat!("Alignment of ", stringify!(Source_c))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Source_c>())).n_points as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(Source_c),
            "::",
            stringify!(n_points)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Source_c>())).point_lmn as *const _ as usize },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(Source_c),
            "::",
            stringify!(point_lmn)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Source_c>())).point_fd as *const _ as usize },
        16usize,
        concat!(
            "Offset of field: ",
            stringify!(Source_c),
            "::",
            stringify!(point_fd)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Source_c>())).n_channels as *const _ as usize },
        24usize,
        concat!(
            "Offset of field: ",
            stringify!(Source_c),
            "::",
            stringify!(n_channels)
        )
    );
}
#[doc = " A container struct for storing visibilities."]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Vis_c {
    #[doc = " The number of visibilities."]
    pub n_vis: ::std::os::raw::c_uint,
    #[doc = " Real components of the visibilities."]
    pub real: *mut f32,
    #[doc = " Imaginary components of the visibilities."]
    pub imag: *mut f32,
}
#[test]
fn bindgen_test_layout_Vis_c() {
    assert_eq!(
        ::std::mem::size_of::<Vis_c>(),
        24usize,
        concat!("Size of: ", stringify!(Vis_c))
    );
    assert_eq!(
        ::std::mem::align_of::<Vis_c>(),
        8usize,
        concat!("Alignment of ", stringify!(Vis_c))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Vis_c>())).n_vis as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(Vis_c),
            "::",
            stringify!(n_vis)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Vis_c>())).real as *const _ as usize },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(Vis_c),
            "::",
            stringify!(real)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Vis_c>())).imag as *const _ as usize },
        16usize,
        concat!(
            "Offset of field: ",
            stringify!(Vis_c),
            "::",
            stringify!(imag)
        )
    );
}
extern "C" {
    pub fn vis_gen(
        uvw: *const UVW_c,
        src: *const Source_c,
        vis: *mut Vis_c,
        n_channels: ::std::os::raw::c_uint,
        n_baselines: ::std::os::raw::c_uint,
    );
}
