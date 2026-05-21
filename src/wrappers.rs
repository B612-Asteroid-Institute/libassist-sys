//! Safe RAII wrappers around ASSIST C objects.

use std::ffi::CString;
use std::path::Path;

use librebound_sys::Simulation;

use crate::ffi;
use crate::{Error, Result};

// ---------------------------------------------------------------------------
// Ephemeris
// ---------------------------------------------------------------------------

/// Owned ASSIST ephemeris data. Freed on drop.
///
/// Read-only after creation — safe to share across threads.
pub struct Ephemeris {
    ptr: *mut ffi::assist_ephem,
}

impl Ephemeris {
    /// Load ephemeris from SPK files.
    pub fn from_paths(planets: &Path, asteroids: &Path) -> Result<Self> {
        let planets_cstr = path_to_cstring(planets)?;
        let asteroids_cstr = path_to_cstring(asteroids)?;
        let ptr =
            unsafe { ffi::assist_ephem_create(planets_cstr.as_ptr(), asteroids_cstr.as_ptr()) };
        if ptr.is_null() {
            return Err(Error::EphemerisError(
                "assist_ephem_create returned null — check file paths".into(),
            ));
        }
        Ok(Self { ptr })
    }

    /// Raw pointer to the underlying `assist_ephem`. Useful for direct FFI calls.
    ///
    /// Returns a `*const` pointer because `Ephemeris` implements `Sync` on the
    /// premise that the underlying data is read-only after construction. Call
    /// `.cast_mut()` at the call site if the target FFI signature requires
    /// `*mut`; that cast is the caller's assertion of unique access.
    pub fn as_ptr(&self) -> *const ffi::assist_ephem {
        self.ptr
    }

    /// Reference Julian date for the ephemeris (typically 2451545.0 = J2000.0 TDB).
    pub fn jd_ref(&self) -> f64 {
        unsafe { ffi::assist_rs_ephem_get_jd_ref(self.ptr) }
    }

    /// Override the reference Julian date.
    ///
    /// Requires `&mut self`, which prevents concurrent mutation when the
    /// `Ephemeris` is shared across threads via `Arc`. Must be called before
    /// any `AssistSim` is attached.
    pub fn set_jd_ref(&mut self, jd: f64) {
        unsafe { ffi::assist_rs_ephem_set_jd_ref(self.ptr, jd) }
    }

    /// Speed of light in AU/day.
    pub fn c_au_per_day(&self) -> f64 {
        unsafe { ffi::assist_rs_ephem_get_c_au_per_day(self.ptr) }
    }

    /// Convert MJD TDB to ASSIST simulation time (days from `jd_ref`).
    ///
    /// ASSIST stores times as days offset from the ephemeris reference JD,
    /// where `JD = MJD + 2_400_000.5`.
    pub fn mjd_to_assist_time(&self, mjd_tdb: f64) -> f64 {
        (mjd_tdb + 2_400_000.5) - self.jd_ref()
    }

    /// Earth equatorial radius in AU.
    pub fn earth_radius_au(&self) -> f64 {
        unsafe { ffi::assist_rs_ephem_get_re_eq(self.ptr) }
    }

    /// Earth/Moon mass ratio.
    pub fn emrat(&self) -> f64 {
        unsafe { ffi::assist_rs_ephem_get_emrat(self.ptr) }
    }

    /// Get a solar system body's state at time `t` (days from `jd_ref`).
    pub fn get_body_state(&self, body_id: i32, t: f64) -> Result<ffi::reb_particle> {
        let mut error: i32 = 0;
        let p = unsafe { ffi::assist_get_particle_with_error(self.ptr, body_id, t, &mut error) };
        if error != 0 {
            return Err(Error::EphemerisError(format!(
                "assist_get_particle failed for body {body_id} at t={t}: error code {error}"
            )));
        }
        Ok(p)
    }

    /// Get a solar system body's 6-element state `[x, y, z, vx, vy, vz]` at
    /// time `t` (days from `jd_ref`). Convenience over [`Self::get_body_state`]
    /// when the caller only needs the kinematic state, not the full
    /// `reb_particle` (mass, hash, etc.).
    pub fn get_body_state_array(&self, body_id: i32, t: f64) -> Result<[f64; 6]> {
        let p = self.get_body_state(body_id, t)?;
        Ok([p.x, p.y, p.z, p.vx, p.vy, p.vz])
    }
}

impl Drop for Ephemeris {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { ffi::assist_ephem_free(self.ptr) }
        }
    }
}

// SAFETY: Ephemeris data is read-only after construction. The underlying
// assist_ephem struct is only mutated during init (assist_ephem_create),
// and all subsequent access (assist_get_particle*) is const-correct in C.
// `set_jd_ref` takes &mut self so Rust's aliasing rules prevent races through
// Arc<Ephemeris>, and `as_ptr` returns *const to forbid back-door mutation.
//
// Note on process-wide concurrency with AssistSim: REBOUND's IAS15 integrator
// and ASSIST force routines use only `static const` tables (no mutable global
// state). The only shared process-global is the SIGINT handler / flag
// (reb_sigint), which all concurrent simulations legitimately share.
// Concurrent AssistSim instances on separate threads are therefore safe.
unsafe impl Send for Ephemeris {}
unsafe impl Sync for Ephemeris {}

// ---------------------------------------------------------------------------
// AssistSim — Simulation with ASSIST forces attached
// ---------------------------------------------------------------------------

/// A REBOUND simulation with ASSIST ephemeris forces attached.
///
/// Owns the simulation. Borrows the ephemeris (caller must keep it alive).
/// ASSIST extras are freed on drop, then the simulation is freed.
pub struct AssistSim {
    sim: Simulation,
    ax: *mut ffi::assist_extras,
    /// Backing storage for ASSIST's `particle_params` pointer. Kept alive
    /// here so its heap buffer lives as long as the simulation.
    particle_params: Option<Vec<f64>>,
}

impl AssistSim {
    /// Create a new ASSIST-powered simulation.
    ///
    /// The `ephem` must outlive this `AssistSim`. ASSIST stores a raw pointer
    /// to the ephemeris data internally.
    pub fn new(mut sim: Simulation, ephem: &Ephemeris) -> Result<Self> {
        let ax = unsafe { ffi::assist_attach(sim.as_mut_ptr(), ephem.ptr) };
        if ax.is_null() {
            return Err(Error::Other("assist_attach returned null".into()));
        }
        // ASSIST sets integrator=IAS15, gravity=NONE, registers force callbacks.
        // Ensure exact finish time is on.
        sim.set_exact_finish_time(true);
        Ok(Self {
            sim,
            ax,
            particle_params: None,
        })
    }

    /// Set the ASSIST force model flags.
    pub fn set_forces(&mut self, flags: i32) {
        unsafe { ffi::assist_rs_extras_set_forces(self.ax, flags) }
    }

    /// Get current force model flags.
    pub fn forces(&self) -> i32 {
        unsafe { ffi::assist_rs_extras_get_forces(self.ax) }
    }

    /// Access the underlying simulation.
    pub fn sim(&self) -> &Simulation {
        &self.sim
    }

    /// Mutable access to the underlying simulation.
    pub fn sim_mut(&mut self) -> &mut Simulation {
        &mut self.sim
    }

    // --- Non-gravitational force model parameters ---

    /// Set the g(r) model exponent α. Default: 1.0.
    pub fn set_alpha(&mut self, v: f64) {
        unsafe { ffi::assist_rs_extras_set_alpha(self.ax, v) }
    }
    pub fn alpha(&self) -> f64 {
        unsafe { ffi::assist_rs_extras_get_alpha(self.ax) }
    }

    /// Set the g(r) model exponent k. Default: 0.0 (pure inverse-power law).
    pub fn set_nk(&mut self, v: f64) {
        unsafe { ffi::assist_rs_extras_set_nk(self.ax, v) }
    }
    pub fn nk(&self) -> f64 {
        unsafe { ffi::assist_rs_extras_get_nk(self.ax) }
    }

    /// Set the g(r) model exponent m. Default: 2.0 (inverse-square).
    pub fn set_nm(&mut self, v: f64) {
        unsafe { ffi::assist_rs_extras_set_nm(self.ax, v) }
    }
    pub fn nm(&self) -> f64 {
        unsafe { ffi::assist_rs_extras_get_nm(self.ax) }
    }

    /// Set the g(r) model exponent n. Default: 5.093 (Marsden-Sekanina water ice).
    pub fn set_nn(&mut self, v: f64) {
        unsafe { ffi::assist_rs_extras_set_nn(self.ax, v) }
    }
    pub fn nn(&self) -> f64 {
        unsafe { ffi::assist_rs_extras_get_nn(self.ax) }
    }

    /// Set the g(r) model scale distance r₀ in AU. Default: 1.0.
    pub fn set_r0(&mut self, v: f64) {
        unsafe { ffi::assist_rs_extras_set_r0(self.ax, v) }
    }
    pub fn r0(&self) -> f64 {
        unsafe { ffi::assist_rs_extras_get_r0(self.ax) }
    }

    /// Install ASSIST's `particle_params` array (3 doubles per particle:
    /// `[A1, A2, A3]`, in `[real | variational]` order).
    ///
    /// Takes ownership of the `Vec`; its heap buffer lives for as long as the
    /// `AssistSim`, matching the lifetime ASSIST requires for the pointer it
    /// stashes internally. Must be called *after* all particles (real +
    /// variational) have been added; `params.len()` must equal
    /// `3 * n_particles`.
    ///
    /// Replacing a previously installed array drops the old storage; the
    /// previous pointer ASSIST held is already overwritten at that point.
    pub fn set_particle_params(&mut self, mut params: Vec<f64>) {
        let n = self.sim.n_particles();
        assert_eq!(
            params.len(),
            3 * n,
            "particle_params length must equal 3 * n_particles (got {}, expected {})",
            params.len(),
            3 * n
        );
        let ptr = params.as_mut_ptr();
        unsafe { ffi::assist_rs_extras_set_particle_params(self.ax, ptr) }
        self.particle_params = Some(params);
    }

    /// Integrate to target time.
    pub fn integrate(&mut self, tmax: f64) -> Result<()> {
        self.sim.integrate(tmax).map_err(Error::from)
    }

    /// Integrate to target time `t`, with interpolation inside the last
    /// completed IAS15 step when possible.
    ///
    /// On first call this behaves like [`integrate`] except it sets
    /// `exact_finish_time = 0` so IAS15 may overshoot; the final state
    /// is reconstructed at `t` via polynomial interpolation using the
    /// integrator's `br` coefficients. On subsequent calls where `t`
    /// falls within the last completed step's interval, no integration
    /// happens at all — pure polynomial evaluation, typically one-to-two
    /// orders of magnitude cheaper than a full step. Intended for
    /// light-time iteration loops where the target shifts by
    /// femtoseconds-to-microseconds per iteration.
    ///
    /// After the call, `sim.particles[i]` holds the state at `t`
    /// (interpolated), even though `sim.t` may be past `t`.
    ///
    /// [`integrate`]: Self::integrate
    pub fn integrate_or_interpolate(&mut self, t: f64) -> Result<()> {
        unsafe { ffi::assist_integrate_or_interpolate(self.ax, t) }
        Ok(())
    }

    /// Raw mutable pointer to the underlying REBOUND simulation.
    /// See [`Simulation::as_mut_ptr`].
    #[doc(hidden)]
    pub fn raw_sim_ptr_mut(&mut self) -> *mut librebound_sys::ffi::reb_simulation {
        self.sim.as_mut_ptr()
    }

    /// Zero IAS15's compensated-summation and predictor state (csx, csv,
    /// csa0, b, e, br, er, g) *in place*, leaving the allocations intact.
    /// Also invalidates the ASSIST ephemeris-lookup cache (sets every slot's
    /// `t` to a sentinel so matched-t comparisons miss on the first
    /// post-reset call). Required between two unrelated orbits integrated on
    /// the same simulation — otherwise stale b/e predictor state seeds the
    /// new orbit's first step and causes extra corrector iterations, and a
    /// populated ephem cache causes per-lookup LRU work that adds up across
    /// ~7000 lookups per 30-day integrate (≈190 µs regression).
    ///
    /// Cheaper than [`librebound_sys::ffi::reb_integrator_ias15_reset`]
    /// (no free/malloc), and faster in practice: a pool-style benchmark with
    /// this helper matches or beats the unpooled free-function path.
    pub fn reset_integrator(&mut self) {
        unsafe {
            librebound_sys::ffi::assist_rs_ias15_zero_state(self.sim.as_mut_ptr());
            ffi::assist_rs_ephem_cache_reset(self.ax);
        }
    }

    /// Rewrite the first three slots of the installed `particle_params`
    /// array (the real test particle's A1, A2, A3) without reallocating.
    /// Returns `Error::Other` if `set_particle_params` was never called —
    /// silently no-op'ing here would let a non-grav orbit keep the previous
    /// orbit's A1/A2/A3 values without any indication.
    ///
    /// The variational-particle parameter columns (indices 3 onward) are
    /// orbit-invariant IC perturbations (identity for parameter
    /// variationals, zero for state variationals) and are left untouched.
    pub fn update_nongrav_coeffs(&mut self, a1: f64, a2: f64, a3: f64) -> Result<()> {
        let params = self.particle_params.as_mut().ok_or_else(|| {
            Error::Other(
                "update_nongrav_coeffs called before set_particle_params; \
                 install a particle_params array first"
                    .into(),
            )
        })?;
        params[0] = a1;
        params[1] = a2;
        params[2] = a3;
        Ok(())
    }
}

impl Drop for AssistSim {
    fn drop(&mut self) {
        if !self.ax.is_null() {
            // Detach ASSIST first, then assist_free, then sim drops automatically.
            unsafe {
                ffi::assist_detach(self.sim.as_mut_ptr(), self.ax);
                ffi::assist_free(self.ax);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn path_to_cstring(path: &Path) -> Result<CString> {
    let s = path.to_str().ok_or_else(|| {
        Error::Other(format!(
            "path contains non-UTF8 characters: {}",
            path.display()
        ))
    })?;
    CString::new(s).map_err(|e| Error::Other(format!("path contains null byte: {e}")))
}
