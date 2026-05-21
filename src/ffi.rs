//! Raw FFI bindings to the ASSIST C library.
//!
//! `assist_ephem` and `assist_extras` are treated as opaque types. Field
//! access goes through thin C helpers compiled in `helpers.c`. REBOUND types
//! (`reb_simulation`, `reb_particle`) come from [`librebound_sys::ffi`] and
//! are re-exported here so ASSIST FFI signatures resolve transparently.

use libc::{c_char, c_double, c_int};

// Re-export every REBOUND FFI item (types, constants, extern fns, accessor
// helpers) so consumers can use a single `libassist_sys::ffi::*` namespace
// for both REBOUND and ASSIST symbols.
pub use librebound_sys::ffi::*;

// ---------------------------------------------------------------------------
// Opaque ASSIST types
// ---------------------------------------------------------------------------

/// Opaque ASSIST ephemeris data.
#[repr(C)]
pub struct assist_ephem {
    _opaque: [u8; 0],
}

/// Opaque ASSIST extras (attaches ASSIST forces to a REBOUND simulation).
#[repr(C)]
pub struct assist_extras {
    _opaque: [u8; 0],
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

// ASSIST body IDs
pub const ASSIST_BODY_SUN: c_int = 0;
pub const ASSIST_BODY_MERCURY: c_int = 1;
pub const ASSIST_BODY_VENUS: c_int = 2;
pub const ASSIST_BODY_EARTH: c_int = 3;
pub const ASSIST_BODY_MOON: c_int = 4;
pub const ASSIST_BODY_MARS: c_int = 5;
pub const ASSIST_BODY_JUPITER: c_int = 6;
pub const ASSIST_BODY_SATURN: c_int = 7;
pub const ASSIST_BODY_URANUS: c_int = 8;
pub const ASSIST_BODY_NEPTUNE: c_int = 9;
pub const ASSIST_BODY_PLUTO: c_int = 10;
pub const ASSIST_BODY_NPLANETS: c_int = 11;

// ASSIST force flags
pub const ASSIST_FORCE_SUN: c_int = 0x01;
pub const ASSIST_FORCE_PLANETS: c_int = 0x02;
pub const ASSIST_FORCE_ASTEROIDS: c_int = 0x04;
pub const ASSIST_FORCE_NON_GRAVITATIONAL: c_int = 0x08;
pub const ASSIST_FORCE_EARTH_HARMONICS: c_int = 0x10;
pub const ASSIST_FORCE_SUN_HARMONICS: c_int = 0x20;
pub const ASSIST_FORCE_GR_EIH: c_int = 0x40;
pub const ASSIST_FORCE_GR_SIMPLE: c_int = 0x80;
pub const ASSIST_FORCE_GR_POTENTIAL: c_int = 0x100;

/// Default force flags: Sun + planets + asteroids + Earth J2/J3/J4 + Sun J2 + GR (EIH).
pub const ASSIST_FORCES_DEFAULT: c_int = ASSIST_FORCE_SUN
    | ASSIST_FORCE_PLANETS
    | ASSIST_FORCE_ASTEROIDS
    | ASSIST_FORCE_EARTH_HARMONICS
    | ASSIST_FORCE_SUN_HARMONICS
    | ASSIST_FORCE_GR_EIH;

// ASSIST status codes
pub const ASSIST_SUCCESS: c_int = 0;
pub const ASSIST_ERROR_EPHEM_FILE: c_int = 1;
pub const ASSIST_ERROR_AST_FILE: c_int = 2;

// ---------------------------------------------------------------------------
// ASSIST functions
// ---------------------------------------------------------------------------

unsafe extern "C" {
    pub fn assist_ephem_create(
        planets_path: *const c_char,
        asteroids_path: *const c_char,
    ) -> *mut assist_ephem;
    pub fn assist_ephem_free(ephem: *mut assist_ephem);

    pub fn assist_attach(sim: *mut reb_simulation, ephem: *mut assist_ephem) -> *mut assist_extras;
    pub fn assist_free(ax: *mut assist_extras);
    pub fn assist_detach(sim: *mut reb_simulation, ax: *mut assist_extras);

    pub fn assist_get_particle(
        ephem: *const assist_ephem,
        particle_id: c_int,
        t: c_double,
    ) -> reb_particle;
    pub fn assist_get_particle_with_error(
        ephem: *const assist_ephem,
        particle_id: c_int,
        t: c_double,
        error: *mut c_int,
    ) -> reb_particle;

    pub fn assist_integrate_or_interpolate(ax: *mut assist_extras, t: c_double);
}

// ---------------------------------------------------------------------------
// C helper functions (from src/helpers.c)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    // Ephem cache reset (between propagations sharing the same simulation).
    pub fn assist_rs_ephem_cache_reset(ax: *mut assist_extras);

    // assist_extras field accessors
    pub fn assist_rs_extras_get_forces(ax: *const assist_extras) -> c_int;
    pub fn assist_rs_extras_set_forces(ax: *mut assist_extras, f: c_int);
    pub fn assist_rs_extras_get_geocentric(ax: *const assist_extras) -> c_int;
    pub fn assist_rs_extras_set_geocentric(ax: *mut assist_extras, g: c_int);
    pub fn assist_rs_extras_get_particle_params(ax: *const assist_extras) -> *mut c_double;
    pub fn assist_rs_extras_set_particle_params(ax: *mut assist_extras, p: *mut c_double);

    // Non-gravitational force model parameters
    pub fn assist_rs_extras_get_alpha(ax: *const assist_extras) -> c_double;
    pub fn assist_rs_extras_set_alpha(ax: *mut assist_extras, v: c_double);
    pub fn assist_rs_extras_get_nk(ax: *const assist_extras) -> c_double;
    pub fn assist_rs_extras_set_nk(ax: *mut assist_extras, v: c_double);
    pub fn assist_rs_extras_get_nm(ax: *const assist_extras) -> c_double;
    pub fn assist_rs_extras_set_nm(ax: *mut assist_extras, v: c_double);
    pub fn assist_rs_extras_get_nn(ax: *const assist_extras) -> c_double;
    pub fn assist_rs_extras_set_nn(ax: *mut assist_extras, v: c_double);
    pub fn assist_rs_extras_get_r0(ax: *const assist_extras) -> c_double;
    pub fn assist_rs_extras_set_r0(ax: *mut assist_extras, v: c_double);

    // assist_ephem field accessors
    pub fn assist_rs_ephem_get_jd_ref(ephem: *const assist_ephem) -> c_double;
    pub fn assist_rs_ephem_set_jd_ref(ephem: *mut assist_ephem, jd: c_double);
    pub fn assist_rs_ephem_get_au(ephem: *const assist_ephem) -> c_double;
    pub fn assist_rs_ephem_get_clight(ephem: *const assist_ephem) -> c_double;
    pub fn assist_rs_ephem_get_c_au_per_day(ephem: *const assist_ephem) -> c_double;
    pub fn assist_rs_ephem_get_re(ephem: *const assist_ephem) -> c_double;
    pub fn assist_rs_ephem_get_re_eq(ephem: *const assist_ephem) -> c_double;
    pub fn assist_rs_ephem_get_emrat(ephem: *const assist_ephem) -> c_double;
}
