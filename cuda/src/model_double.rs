/* automatically generated by rust-bindgen 0.59.1 */

pub type __uint64_t = ::std::os::raw::c_ulong;
#[doc = " The (u,v,w) coordinates of a baseline. They are in units of metres."]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct UVW {
    pub u: f64,
    pub v: f64,
    pub w: f64,
}
#[test]
fn bindgen_test_layout_UVW() {
    assert_eq!(
        ::std::mem::size_of::<UVW>(),
        24usize,
        concat!("Size of: ", stringify!(UVW))
    );
    assert_eq!(
        ::std::mem::align_of::<UVW>(),
        8usize,
        concat!("Alignment of ", stringify!(UVW))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<UVW>())).u as *const _ as usize },
        0usize,
        concat!("Offset of field: ", stringify!(UVW), "::", stringify!(u))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<UVW>())).v as *const _ as usize },
        8usize,
        concat!("Offset of field: ", stringify!(UVW), "::", stringify!(v))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<UVW>())).w as *const _ as usize },
        16usize,
        concat!("Offset of field: ", stringify!(UVW), "::", stringify!(w))
    );
}
#[doc = " The LMN coordinates of a sky-model component."]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct LMN {
    pub l: f64,
    pub m: f64,
    pub n: f64,
}
#[test]
fn bindgen_test_layout_LMN() {
    assert_eq!(
        ::std::mem::size_of::<LMN>(),
        24usize,
        concat!("Size of: ", stringify!(LMN))
    );
    assert_eq!(
        ::std::mem::align_of::<LMN>(),
        8usize,
        concat!("Alignment of ", stringify!(LMN))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<LMN>())).l as *const _ as usize },
        0usize,
        concat!("Offset of field: ", stringify!(LMN), "::", stringify!(l))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<LMN>())).m as *const _ as usize },
        8usize,
        concat!("Offset of field: ", stringify!(LMN), "::", stringify!(m))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<LMN>())).n as *const _ as usize },
        16usize,
        concat!("Offset of field: ", stringify!(LMN), "::", stringify!(n))
    );
}
#[doc = " Parameters describing a Gaussian (also applicable to shapelets)."]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GaussianParams {
    pub maj: f64,
    pub min: f64,
    pub pa: f64,
}
#[test]
fn bindgen_test_layout_GaussianParams() {
    assert_eq!(
        ::std::mem::size_of::<GaussianParams>(),
        24usize,
        concat!("Size of: ", stringify!(GaussianParams))
    );
    assert_eq!(
        ::std::mem::align_of::<GaussianParams>(),
        8usize,
        concat!("Alignment of ", stringify!(GaussianParams))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<GaussianParams>())).maj as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(GaussianParams),
            "::",
            stringify!(maj)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<GaussianParams>())).min as *const _ as usize },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(GaussianParams),
            "::",
            stringify!(min)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<GaussianParams>())).pa as *const _ as usize },
        16usize,
        concat!(
            "Offset of field: ",
            stringify!(GaussianParams),
            "::",
            stringify!(pa)
        )
    );
}
#[doc = " Parameters describing a shapelet coefficient."]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ShapeletCoeff {
    pub n1: usize,
    pub n2: usize,
    pub value: f64,
}
#[test]
fn bindgen_test_layout_ShapeletCoeff() {
    assert_eq!(
        ::std::mem::size_of::<ShapeletCoeff>(),
        24usize,
        concat!("Size of: ", stringify!(ShapeletCoeff))
    );
    assert_eq!(
        ::std::mem::align_of::<ShapeletCoeff>(),
        8usize,
        concat!("Alignment of ", stringify!(ShapeletCoeff))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<ShapeletCoeff>())).n1 as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(ShapeletCoeff),
            "::",
            stringify!(n1)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<ShapeletCoeff>())).n2 as *const _ as usize },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(ShapeletCoeff),
            "::",
            stringify!(n2)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<ShapeletCoeff>())).value as *const _ as usize },
        16usize,
        concat!(
            "Offset of field: ",
            stringify!(ShapeletCoeff),
            "::",
            stringify!(value)
        )
    );
}
#[doc = " (u,v) coordinates for a shapelet. W isn't used, so we're a bit more efficient"]
#[doc = " by not using UVW."]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ShapeletUV {
    pub u: f64,
    pub v: f64,
}
#[test]
fn bindgen_test_layout_ShapeletUV() {
    assert_eq!(
        ::std::mem::size_of::<ShapeletUV>(),
        16usize,
        concat!("Size of: ", stringify!(ShapeletUV))
    );
    assert_eq!(
        ::std::mem::align_of::<ShapeletUV>(),
        8usize,
        concat!("Alignment of ", stringify!(ShapeletUV))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<ShapeletUV>())).u as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(ShapeletUV),
            "::",
            stringify!(u)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<ShapeletUV>())).v as *const _ as usize },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(ShapeletUV),
            "::",
            stringify!(v)
        )
    );
}
#[doc = " A Jones matrix, single precision. The floats are unpacked into real and imag"]
#[doc = " components because complex numbers don't traverse the FFI boundary well."]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct JonesF32 {
    pub xx_re: f32,
    pub xx_im: f32,
    pub xy_re: f32,
    pub xy_im: f32,
    pub yx_re: f32,
    pub yx_im: f32,
    pub yy_re: f32,
    pub yy_im: f32,
}
#[test]
fn bindgen_test_layout_JonesF32() {
    assert_eq!(
        ::std::mem::size_of::<JonesF32>(),
        32usize,
        concat!("Size of: ", stringify!(JonesF32))
    );
    assert_eq!(
        ::std::mem::align_of::<JonesF32>(),
        4usize,
        concat!("Alignment of ", stringify!(JonesF32))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<JonesF32>())).xx_re as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(JonesF32),
            "::",
            stringify!(xx_re)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<JonesF32>())).xx_im as *const _ as usize },
        4usize,
        concat!(
            "Offset of field: ",
            stringify!(JonesF32),
            "::",
            stringify!(xx_im)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<JonesF32>())).xy_re as *const _ as usize },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(JonesF32),
            "::",
            stringify!(xy_re)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<JonesF32>())).xy_im as *const _ as usize },
        12usize,
        concat!(
            "Offset of field: ",
            stringify!(JonesF32),
            "::",
            stringify!(xy_im)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<JonesF32>())).yx_re as *const _ as usize },
        16usize,
        concat!(
            "Offset of field: ",
            stringify!(JonesF32),
            "::",
            stringify!(yx_re)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<JonesF32>())).yx_im as *const _ as usize },
        20usize,
        concat!(
            "Offset of field: ",
            stringify!(JonesF32),
            "::",
            stringify!(yx_im)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<JonesF32>())).yy_re as *const _ as usize },
        24usize,
        concat!(
            "Offset of field: ",
            stringify!(JonesF32),
            "::",
            stringify!(yy_re)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<JonesF32>())).yy_im as *const _ as usize },
        28usize,
        concat!(
            "Offset of field: ",
            stringify!(JonesF32),
            "::",
            stringify!(yy_im)
        )
    );
}
#[doc = " A Jones matrix, double precision. The floats are unpacked into real and imag"]
#[doc = " components because complex numbers don't traverse the FFI boundary well."]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct JonesF64 {
    pub xx_re: f64,
    pub xx_im: f64,
    pub xy_re: f64,
    pub xy_im: f64,
    pub yx_re: f64,
    pub yx_im: f64,
    pub yy_re: f64,
    pub yy_im: f64,
}
#[test]
fn bindgen_test_layout_JonesF64() {
    assert_eq!(
        ::std::mem::size_of::<JonesF64>(),
        64usize,
        concat!("Size of: ", stringify!(JonesF64))
    );
    assert_eq!(
        ::std::mem::align_of::<JonesF64>(),
        8usize,
        concat!("Alignment of ", stringify!(JonesF64))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<JonesF64>())).xx_re as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(JonesF64),
            "::",
            stringify!(xx_re)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<JonesF64>())).xx_im as *const _ as usize },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(JonesF64),
            "::",
            stringify!(xx_im)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<JonesF64>())).xy_re as *const _ as usize },
        16usize,
        concat!(
            "Offset of field: ",
            stringify!(JonesF64),
            "::",
            stringify!(xy_re)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<JonesF64>())).xy_im as *const _ as usize },
        24usize,
        concat!(
            "Offset of field: ",
            stringify!(JonesF64),
            "::",
            stringify!(xy_im)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<JonesF64>())).yx_re as *const _ as usize },
        32usize,
        concat!(
            "Offset of field: ",
            stringify!(JonesF64),
            "::",
            stringify!(yx_re)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<JonesF64>())).yx_im as *const _ as usize },
        40usize,
        concat!(
            "Offset of field: ",
            stringify!(JonesF64),
            "::",
            stringify!(yx_im)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<JonesF64>())).yy_re as *const _ as usize },
        48usize,
        concat!(
            "Offset of field: ",
            stringify!(JonesF64),
            "::",
            stringify!(yy_re)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<JonesF64>())).yy_im as *const _ as usize },
        56usize,
        concat!(
            "Offset of field: ",
            stringify!(JonesF64),
            "::",
            stringify!(yy_im)
        )
    );
}
#[doc = " All the parameters needed to describe point-source components."]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Points {
    pub num_power_law_points: usize,
    pub power_law_lmns: *mut LMN,
    pub power_law_fds: *mut JonesF64,
    pub power_law_sis: *mut f64,
    pub num_list_points: usize,
    pub list_lmns: *mut LMN,
    pub list_fds: *mut JonesF64,
}
#[test]
fn bindgen_test_layout_Points() {
    assert_eq!(
        ::std::mem::size_of::<Points>(),
        56usize,
        concat!("Size of: ", stringify!(Points))
    );
    assert_eq!(
        ::std::mem::align_of::<Points>(),
        8usize,
        concat!("Alignment of ", stringify!(Points))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Points>())).num_power_law_points as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(Points),
            "::",
            stringify!(num_power_law_points)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Points>())).power_law_lmns as *const _ as usize },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(Points),
            "::",
            stringify!(power_law_lmns)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Points>())).power_law_fds as *const _ as usize },
        16usize,
        concat!(
            "Offset of field: ",
            stringify!(Points),
            "::",
            stringify!(power_law_fds)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Points>())).power_law_sis as *const _ as usize },
        24usize,
        concat!(
            "Offset of field: ",
            stringify!(Points),
            "::",
            stringify!(power_law_sis)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Points>())).num_list_points as *const _ as usize },
        32usize,
        concat!(
            "Offset of field: ",
            stringify!(Points),
            "::",
            stringify!(num_list_points)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Points>())).list_lmns as *const _ as usize },
        40usize,
        concat!(
            "Offset of field: ",
            stringify!(Points),
            "::",
            stringify!(list_lmns)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Points>())).list_fds as *const _ as usize },
        48usize,
        concat!(
            "Offset of field: ",
            stringify!(Points),
            "::",
            stringify!(list_fds)
        )
    );
}
#[doc = " All the parameters needed to describe Gaussian components."]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Gaussians {
    pub num_power_law_gaussians: usize,
    pub power_law_lmns: *mut LMN,
    pub power_law_fds: *mut JonesF64,
    pub power_law_sis: *mut f64,
    pub power_law_gps: *mut GaussianParams,
    pub num_list_gaussians: usize,
    pub list_lmns: *mut LMN,
    pub list_fds: *mut JonesF64,
    pub list_gps: *mut GaussianParams,
}
#[test]
fn bindgen_test_layout_Gaussians() {
    assert_eq!(
        ::std::mem::size_of::<Gaussians>(),
        72usize,
        concat!("Size of: ", stringify!(Gaussians))
    );
    assert_eq!(
        ::std::mem::align_of::<Gaussians>(),
        8usize,
        concat!("Alignment of ", stringify!(Gaussians))
    );
    assert_eq!(
        unsafe {
            &(*(::std::ptr::null::<Gaussians>())).num_power_law_gaussians as *const _ as usize
        },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(Gaussians),
            "::",
            stringify!(num_power_law_gaussians)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Gaussians>())).power_law_lmns as *const _ as usize },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(Gaussians),
            "::",
            stringify!(power_law_lmns)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Gaussians>())).power_law_fds as *const _ as usize },
        16usize,
        concat!(
            "Offset of field: ",
            stringify!(Gaussians),
            "::",
            stringify!(power_law_fds)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Gaussians>())).power_law_sis as *const _ as usize },
        24usize,
        concat!(
            "Offset of field: ",
            stringify!(Gaussians),
            "::",
            stringify!(power_law_sis)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Gaussians>())).power_law_gps as *const _ as usize },
        32usize,
        concat!(
            "Offset of field: ",
            stringify!(Gaussians),
            "::",
            stringify!(power_law_gps)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Gaussians>())).num_list_gaussians as *const _ as usize },
        40usize,
        concat!(
            "Offset of field: ",
            stringify!(Gaussians),
            "::",
            stringify!(num_list_gaussians)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Gaussians>())).list_lmns as *const _ as usize },
        48usize,
        concat!(
            "Offset of field: ",
            stringify!(Gaussians),
            "::",
            stringify!(list_lmns)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Gaussians>())).list_fds as *const _ as usize },
        56usize,
        concat!(
            "Offset of field: ",
            stringify!(Gaussians),
            "::",
            stringify!(list_fds)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Gaussians>())).list_gps as *const _ as usize },
        64usize,
        concat!(
            "Offset of field: ",
            stringify!(Gaussians),
            "::",
            stringify!(list_gps)
        )
    );
}
#[doc = " All the parameters needed to describe Shapelet components."]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Shapelets {
    pub num_power_law_shapelets: usize,
    pub power_law_lmns: *mut LMN,
    pub power_law_fds: *mut JonesF64,
    pub power_law_sis: *mut f64,
    pub power_law_gps: *mut GaussianParams,
    pub power_law_shapelet_uvs: *mut ShapeletUV,
    pub power_law_shapelet_coeffs: *mut ShapeletCoeff,
    pub power_law_num_shapelet_coeffs: *mut usize,
    pub num_list_shapelets: usize,
    pub list_lmns: *mut LMN,
    pub list_fds: *mut JonesF64,
    pub list_gps: *mut GaussianParams,
    pub list_shapelet_uvs: *mut ShapeletUV,
    pub list_shapelet_coeffs: *mut ShapeletCoeff,
    pub list_num_shapelet_coeffs: *mut usize,
}
#[test]
fn bindgen_test_layout_Shapelets() {
    assert_eq!(
        ::std::mem::size_of::<Shapelets>(),
        120usize,
        concat!("Size of: ", stringify!(Shapelets))
    );
    assert_eq!(
        ::std::mem::align_of::<Shapelets>(),
        8usize,
        concat!("Alignment of ", stringify!(Shapelets))
    );
    assert_eq!(
        unsafe {
            &(*(::std::ptr::null::<Shapelets>())).num_power_law_shapelets as *const _ as usize
        },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(Shapelets),
            "::",
            stringify!(num_power_law_shapelets)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Shapelets>())).power_law_lmns as *const _ as usize },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(Shapelets),
            "::",
            stringify!(power_law_lmns)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Shapelets>())).power_law_fds as *const _ as usize },
        16usize,
        concat!(
            "Offset of field: ",
            stringify!(Shapelets),
            "::",
            stringify!(power_law_fds)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Shapelets>())).power_law_sis as *const _ as usize },
        24usize,
        concat!(
            "Offset of field: ",
            stringify!(Shapelets),
            "::",
            stringify!(power_law_sis)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Shapelets>())).power_law_gps as *const _ as usize },
        32usize,
        concat!(
            "Offset of field: ",
            stringify!(Shapelets),
            "::",
            stringify!(power_law_gps)
        )
    );
    assert_eq!(
        unsafe {
            &(*(::std::ptr::null::<Shapelets>())).power_law_shapelet_uvs as *const _ as usize
        },
        40usize,
        concat!(
            "Offset of field: ",
            stringify!(Shapelets),
            "::",
            stringify!(power_law_shapelet_uvs)
        )
    );
    assert_eq!(
        unsafe {
            &(*(::std::ptr::null::<Shapelets>())).power_law_shapelet_coeffs as *const _ as usize
        },
        48usize,
        concat!(
            "Offset of field: ",
            stringify!(Shapelets),
            "::",
            stringify!(power_law_shapelet_coeffs)
        )
    );
    assert_eq!(
        unsafe {
            &(*(::std::ptr::null::<Shapelets>())).power_law_num_shapelet_coeffs as *const _ as usize
        },
        56usize,
        concat!(
            "Offset of field: ",
            stringify!(Shapelets),
            "::",
            stringify!(power_law_num_shapelet_coeffs)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Shapelets>())).num_list_shapelets as *const _ as usize },
        64usize,
        concat!(
            "Offset of field: ",
            stringify!(Shapelets),
            "::",
            stringify!(num_list_shapelets)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Shapelets>())).list_lmns as *const _ as usize },
        72usize,
        concat!(
            "Offset of field: ",
            stringify!(Shapelets),
            "::",
            stringify!(list_lmns)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Shapelets>())).list_fds as *const _ as usize },
        80usize,
        concat!(
            "Offset of field: ",
            stringify!(Shapelets),
            "::",
            stringify!(list_fds)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Shapelets>())).list_gps as *const _ as usize },
        88usize,
        concat!(
            "Offset of field: ",
            stringify!(Shapelets),
            "::",
            stringify!(list_gps)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Shapelets>())).list_shapelet_uvs as *const _ as usize },
        96usize,
        concat!(
            "Offset of field: ",
            stringify!(Shapelets),
            "::",
            stringify!(list_shapelet_uvs)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<Shapelets>())).list_shapelet_coeffs as *const _ as usize },
        104usize,
        concat!(
            "Offset of field: ",
            stringify!(Shapelets),
            "::",
            stringify!(list_shapelet_coeffs)
        )
    );
    assert_eq!(
        unsafe {
            &(*(::std::ptr::null::<Shapelets>())).list_num_shapelet_coeffs as *const _ as usize
        },
        112usize,
        concat!(
            "Offset of field: ",
            stringify!(Shapelets),
            "::",
            stringify!(list_num_shapelet_coeffs)
        )
    );
}
pub const POWER_LAW_FD_REF_FREQ: f64 = 150000000.0;
extern "C" {
    #[doc = " Generate sky-model visibilities for a single timestep given multiple"]
    #[doc = " sky-model point-source components. See the documentation of `model_timestep`"]
    #[doc = " for more info."]
    pub fn model_points(
        points: *const Points,
        a: *const Addresses,
        d_uvws: *const UVW,
        d_beam_jones: *const ::std::os::raw::c_void,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    #[doc = " Generate sky-model visibilities for a single timestep given multiple"]
    #[doc = " sky-model Gaussian components. See the documentation of `model_timestep` for"]
    #[doc = " more info."]
    pub fn model_gaussians(
        gaussians: *const Gaussians,
        a: *const Addresses,
        d_uvws: *const UVW,
        d_beam_jones: *const ::std::os::raw::c_void,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    #[doc = " Generate sky-model visibilities for a single timestep given multiple"]
    #[doc = " sky-model shapelet components. See the documentation of `model_timestep` for"]
    #[doc = " more info."]
    pub fn model_shapelets(
        shapelets: *const Shapelets,
        a: *const Addresses,
        d_uvws: *const UVW,
        d_beam_jones: *const ::std::os::raw::c_void,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    #[doc = " Generate sky-model visibilities for a single timestep given multiple"]
    #[doc = " sky-model sources."]
    #[doc = ""]
    #[doc = " `uvws` has one element per baseline. `freqs` has one element per..."]
    #[doc = " frequency."]
    #[doc = ""]
    #[doc = " `points`, `gaussians` and `shapelets` contain coordinates and flux densities"]
    #[doc = " for their respective component types. The components are further split into"]
    #[doc = " \"power law\" and \"list\" types; this is done for efficiency. For the list"]
    #[doc = " types, the flux densities (\"fds\") are two-dimensional arrays, of which the"]
    #[doc = " first axis corresponds to frequency and the second component."]
    #[doc = ""]
    #[doc = " `*_shapelet_uvs` are special UVWs (without the Ws) calculated just for the"]
    #[doc = " shapelets. These are two-dimensional arrays; the first axis corresponds to"]
    #[doc = " baselines and the second a shapelet component."]
    #[doc = ""]
    #[doc = " `*_shapelet_coeffs` is a flattened array-of-arrays. The length of each"]
    #[doc = " sub-array is indicated by `*_num_shapelet_coeffs` (which has a length equal"]
    #[doc = " to `*_num_shapelets`)."]
    #[doc = ""]
    #[doc = " `vis` is a two-dimensional array, of which the first axis corresponds to"]
    #[doc = " baselines and the second frequency. It is the only argument that should be"]
    #[doc = " mutated and should be completely full of zeros before this function is"]
    #[doc = " called."]
    pub fn model_timestep_no_beam(
        num_baselines: ::std::os::raw::c_int,
        num_freqs: ::std::os::raw::c_int,
        uvws: *mut UVW,
        freqs: *mut f64,
        points: *mut Points,
        gaussians: *mut Gaussians,
        shapelets: *mut Shapelets,
        shapelet_basis_values: *mut f64,
        sbf_l: ::std::os::raw::c_int,
        sbf_n: ::std::os::raw::c_int,
        sbf_c: f64,
        sbf_dx: f64,
        vis: *mut JonesF32,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn model_timestep_fee_beam(
        num_baselines: ::std::os::raw::c_int,
        num_freqs: ::std::os::raw::c_int,
        num_tiles: ::std::os::raw::c_int,
        uvws: *mut UVW,
        freqs: *mut f64,
        points: *mut Points,
        gaussians: *mut Gaussians,
        shapelets: *mut Shapelets,
        shapelet_basis_values: *mut f64,
        sbf_l: ::std::os::raw::c_int,
        sbf_n: ::std::os::raw::c_int,
        sbf_c: f64,
        sbf_dx: f64,
        d_beam_coeffs: *mut ::std::os::raw::c_void,
        num_beam_coeffs: ::std::os::raw::c_int,
        num_unique_fee_tiles: ::std::os::raw::c_int,
        num_unique_fee_freqs: ::std::os::raw::c_int,
        d_beam_jones_map: *mut u64,
        d_beam_norm_jones: *mut ::std::os::raw::c_void,
        vis: *mut JonesF32,
    ) -> ::std::os::raw::c_int;
}
