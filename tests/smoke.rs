//! FFI smoke tests — exercise the ASSIST C library.
//!
//! Tests that need a real Ephemeris look for SPK files via:
//!   ASSIST_PLANETS_PATH    — path to de440.bsp
//!   ASSIST_ASTEROIDS_PATH  — path to sb441-n16.bsp
//! Tests that need ephem data skip cleanly when those env vars are unset.

use libassist_sys::{AssistSim, Ephemeris, IntegratorConfig, Simulation};
use std::path::PathBuf;

fn load_ephem() -> Option<Ephemeris> {
    let planets = std::env::var("ASSIST_PLANETS_PATH").ok()?;
    let asteroids = std::env::var("ASSIST_ASTEROIDS_PATH").ok()?;
    Ephemeris::from_paths(&PathBuf::from(planets), &PathBuf::from(asteroids)).ok()
}

#[test]
fn ephemeris_missing_files_returns_error() {
    let result = Ephemeris::from_paths(
        std::path::Path::new("/definitely/does/not/exist.bsp"),
        std::path::Path::new("/also/missing.bsp"),
    );
    let err = match result {
        Ok(_) => panic!("expected error from missing ephemeris files"),
        Err(e) => e,
    };
    assert!(matches!(err, libassist_sys::Error::EphemerisError(_)));
}

/// `AssistSim::update_nongrav_coeffs` used to silently no-op when
/// `set_particle_params` had not been called — meaning a downstream caller
/// could think they'd installed non-grav coefficients while in fact the
/// underlying particle_params array was missing, and the simulation kept
/// whatever (zero) values it had. The wrapper now returns
/// `Error::Other(...)` so the misuse surfaces immediately.
#[test]
fn update_nongrav_coeffs_errors_when_params_uninstalled() {
    let Some(ephem) = load_ephem() else {
        eprintln!("Skipping: ephemeris not available");
        return;
    };

    let mut sim = Simulation::new().unwrap();
    IntegratorConfig::default().apply(&mut sim);
    let mut asim = AssistSim::new(sim, &ephem).expect("attach ASSIST");

    // Deliberately skip set_particle_params; update should now error.
    let result = asim.update_nongrav_coeffs(1e-10, 2e-10, 0.0);
    match result {
        Err(libassist_sys::Error::Other(msg)) => {
            assert!(
                msg.contains("set_particle_params"),
                "error should mention set_particle_params; got: {msg}"
            );
        }
        other => panic!("expected Error::Other, got {other:?}"),
    }
}
