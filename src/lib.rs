//! Raw FFI bindings and safe RAII wrappers for ASSIST.
//!
//! ASSIST is a C library for ephemeris-quality integration of test particles
//! in the solar system, built on top of the REBOUND N-body code. This crate
//! exposes the underlying C API as:
//!
//! - [`ffi`]: raw `extern "C"` bindings to ASSIST functions + types.
//! - [`Ephemeris`]: thin, allocation-owning RAII wrapper around `assist_ephem`.
//! - [`AssistSim`]: a REBOUND simulation with ASSIST forces attached.
//!
//! This crate depends on `librebound-sys` for the REBOUND FFI types
//! (`reb_simulation`, `reb_particle`, [`Simulation`], etc.) and re-exports
//! them at the crate root so downstream consumers can use a single
//! `libassist-sys` import.
//!
//! Higher-level domain logic (orbital-element conversions, observatory
//! handling, light-time iteration, STM propagation, data downloading) lives
//! in the companion `assist-rs` crate, which depends on this one.

pub mod ffi;
mod wrappers;

pub use wrappers::{AssistSim, Ephemeris};

// Re-export REBOUND symbols so downstream consumers can keep a single import.
pub use librebound_sys::{Ias15AdaptiveMode, IntegratorConfig, Simulation};

/// Errors produced by the ASSIST FFI wrappers.
///
/// Wraps [`librebound_sys::Error`] for REBOUND-level failures and adds the
/// ASSIST-specific ephemeris error variant. Higher-level errors (light-time
/// convergence, observatory lookup, etc.) live in `assist_rs::Error`, which
/// wraps this type.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Wrapped REBOUND error from librebound-sys.
    #[error(transparent)]
    Reb(#[from] librebound_sys::Error),

    /// ASSIST ephemeris failure (missing file, malformed data, etc.).
    #[error("ASSIST ephemeris error: {0}")]
    EphemerisError(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
