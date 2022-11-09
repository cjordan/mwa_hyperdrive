/* automatically generated by rust-bindgen 0.68.1 */

pub const POWER_LAW_FD_REF_FREQ: f64 = 150000000.0;
extern "C" {
    #[doc = " Generate sky-model visibilities for a single timestep given multiple\n sky-model point-source components.\n\n `points` contains coordinates and flux densities for their respective\n component types. The components are further split into \"power law\", \"curved\n power law\" and \"list\" types; this is done for efficiency. For the list types,\n the flux densities (\"fds\") are two-dimensional arrays, of which the first\n axis corresponds to frequency and the second component.\n\n `a` is the populated `Addresses` struct needed to do any sky modelling.\n\n `d_uvws` has one element per baseline.\n\n `d_beam_jones` is the beam response used for each unique tile, unique\n frequency and direction. The metadata within `a` allows disambiguation of\n which tile and frequency should use which set of responses."]
    pub fn model_points(
        comps: *const Points,
        a: *const Addresses,
        d_uvws: *const UVW,
        d_beam_jones: *const JonesF64,
        d_vis_fb: *mut JonesF32,
    ) -> *const ::std::os::raw::c_char;
}
extern "C" {
    #[doc = " Generate sky-model visibilities for a single timestep given multiple\n sky-model Gaussian components. See the documentation of `model_points` for\n more info."]
    pub fn model_gaussians(
        comps: *const Gaussians,
        a: *const Addresses,
        d_uvws: *const UVW,
        d_beam_jones: *const JonesF64,
        d_vis_fb: *mut JonesF32,
    ) -> *const ::std::os::raw::c_char;
}
extern "C" {
    #[doc = " Generate sky-model visibilities for a single timestep given multiple\n sky-model shapelet components. See the documentation of `model_points` for\n more info."]
    pub fn model_shapelets(
        comps: *const Shapelets,
        a: *const Addresses,
        d_uvws: *const UVW,
        d_beam_jones: *const JonesF64,
        d_vis_fb: *mut JonesF32,
    ) -> *const ::std::os::raw::c_char;
}
