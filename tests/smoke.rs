//! FFI smoke tests — exercise the ASSIST C library without requiring real
//! ephemeris data files.

use libassist_sys::Ephemeris;

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
