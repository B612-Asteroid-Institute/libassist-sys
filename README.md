# libassist-sys: Raw FFI bindings and safe RAII wrappers for the ASSIST ephemeris-driven N-body integrator
#### A Rust crate by the Asteroid Institute, a program of the B612 Foundation

[![REBOUND 4.6.0](https://img.shields.io/badge/REBOUND-4.6.0-orange?style=flat-square)](https://github.com/hannorein/rebound)
[![ASSIST 1.2.0](https://img.shields.io/badge/ASSIST-1.2.0-orange?style=flat-square)](https://github.com/matthewholman/assist)<br/>
[![CI](https://github.com/B612-Asteroid-Institute/libassist-sys/actions/workflows/rust.yml/badge.svg?style=flat-square)](https://github.com/B612-Asteroid-Institute/libassist-sys/actions/workflows/rust.yml)
[![crates.io](https://img.shields.io/crates/v/libassist-sys.svg?style=flat-square)](https://crates.io/crates/libassist-sys)
[![docs.rs](https://img.shields.io/docsrs/libassist-sys?style=flat-square)](https://docs.rs/libassist-sys)
[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-blue.svg?style=flat-square)](LICENSE)<br/>
[![GitHub](https://img.shields.io/badge/GitHub-B612--Asteroid--Institute-181717?style=flat-square&logo=github&logoColor=white)](https://github.com/B612-Asteroid-Institute)
[![Website](https://img.shields.io/badge/Website-asteroid.institute-1f6feb?style=flat-square&logo=googlechrome&logoColor=white)](https://asteroid.institute/)

`libassist-sys` is a `-sys` crate exposing the C ABI of [ASSIST](https://github.com/matthewholman/assist), JPL's ephemeris-driven N-body force model layered on top of [REBOUND](https://github.com/hannorein/rebound). It vendors ASSIST as a git submodule, compiles it via the `cc` crate, and provides:

- `ffi` — raw `extern "C"` bindings to ASSIST functions and types.
- `Ephemeris` — allocation-owning RAII wrapper around `assist_ephem`. Loads `de440.bsp` + `sb441-n16.bsp`. `Send + Sync` — load once, share across threads.
- `AssistSim` — a REBOUND `Simulation` with ASSIST forces attached. Owns its `assist_extras`, detaches and frees on drop. Supports non-grav coefficients (Marsden-Sekanina), variational equations for STM, and `integrate_or_interpolate` for cheap light-time iteration.
- `Error` — wraps REBOUND's typed integration-exit conditions plus an `EphemerisError` variant for SPK / lookup failures.

Force model (ASSIST defaults): Sun + 8 planets + Moon + 16 massive asteroids + J2 Earth + general relativity. Optional non-gravitational acceleration.

No domain logic — no orbital elements, no observatories, no light-time iteration. Layer those on top via [`assist-rs`](https://crates.io/crates/assist-rs).

## Crate hierarchy

```text
assist-rs            ← domain types: Orbit, Origin, Observer, DataManager
  └── libassist-sys  ← raw ASSIST FFI + AssistSim/Ephemeris RAII (this crate)
        └── librebound-sys  ← raw REBOUND FFI + Simulation RAII
```

## Installation

```toml
[dependencies]
libassist-sys = "1.2"
```

Requires a C compiler (clang recommended for ThinLTO, see [Building](#building)). The crate depends on [`librebound-sys`](https://crates.io/crates/librebound-sys) for the REBOUND ABI.

## Usage

```rust
use libassist_sys::{AssistSim, Ephemeris, IntegratorConfig, Simulation, ffi};
use std::path::Path;

let ephem = Ephemeris::from_paths(
    Path::new("/path/to/de440.bsp"),
    Path::new("/path/to/sb441-n16.bsp"),
)?;

let mut sim = Simulation::new()?;
sim.set_t(0.0);
IntegratorConfig::default().apply(&mut sim);

let mut asim = AssistSim::new(sim, &ephem)?;
asim.set_forces(ffi::ASSIST_FORCES_DEFAULT);

// Add a test particle in barycentric equatorial ICRF (AU, AU/day).
asim.sim_mut().add_test_particle(
    -1.938_169_72, 2.289_213_79, 1.094_048_30,
    -0.008_744_54, -0.005_523_16, 0.001_174_22,
);

let t_target = ephem.mjd_to_assist_time(60030.0);
asim.integrate(t_target)?;
let state = &asim.sim().particles()[0];
println!("x = {:.6} AU", state.x);
# Ok::<(), libassist_sys::Error>(())
```

For typed `Orbit` + heliocentric ecliptic API, light-time correction, observatories, and data-file management, use [`assist-rs`](https://crates.io/crates/assist-rs) instead — it re-exports everything from this crate.

## Thread safety

`Ephemeris` is `Send + Sync`: the underlying `assist_ephem` is read-only after construction, the only mutator (`set_jd_ref`) takes `&mut self`, and `as_ptr()` returns `*const` to forbid back-door mutation. Multiple `AssistSim` instances on separate threads sharing one `Arc<Ephemeris>` is supported — REBOUND's hot paths use only `static const` tables, and the only process-shared mutable state is the SIGINT handler, which all simulations legitimately share. See `wrappers.rs` for the full audit.

`AssistSim` is `Send` but not `Sync` — one mutable simulation per thread.

## Building

The crate vendors ASSIST under `vendor/assist/` (git submodule). `build.rs` compiles `vendor/assist/src/*.c` against `librebound-sys`'s REBOUND headers via the `cc` crate.

```bash
git clone --recursive <repo-url>
cargo build --release
```

For maximum performance use `clang` (honours `-flto=thin`):

```bash
CC=clang cargo build --release
```

GCC builds are correct but ~5–7 % slower on hot ephemeris/force-evaluation paths.

## Testing

```bash
export ASSIST_PLANETS_PATH=/path/to/de440.bsp
export ASSIST_ASTEROIDS_PATH=/path/to/sb441-n16.bsp
cargo test
```

Tests that need ephemeris data skip cleanly when the environment variables are unset. SPK files are available from the [B612 Asteroid Institute data packages](https://b612.ai/opensource/data_packages/) or directly from JPL.

## Versioning

The crate's `major.minor` mirrors the vendored ASSIST release: **libassist-sys 1.2.x wraps ASSIST 1.2.0**. The patch component is reserved for Rust-side fixes that don't change the underlying C library. An ASSIST 1.3 release would become libassist-sys 1.3.0; an ASSIST 2.0 release would become libassist-sys 2.0.0.

The REBOUND version is whatever [`librebound-sys`](https://crates.io/crates/librebound-sys) pulls in (currently 4.6.0). REBOUND-only upgrades that don't bump ASSIST will bump only librebound-sys; libassist-sys may follow with a patch release if any of its wrappers need adjustment.

### Why this scheme

`-sys` crates are thin wrappers around a single upstream library. Mirroring upstream's `major.minor` makes the dependency obvious from the version alone — no need to consult a mapping table or `[package.metadata.vendored]`. The Rust side gets the patch slot for wrapper fixes (e.g. tightening Send/Sync bounds, fixing a leak in a wrapper, adding a typed Error variant), which is where ~all non-upstream changes land in practice.

This is **not** standard Rust semver — bumping the crate's major version isn't an intrinsic Rust-API break; it tracks the C ABI break in the upstream library. Callers depending on `libassist-sys = "1"` get any 1.x.y; callers needing strict pinning should pin to `1.2` or `=1.2.0`.

| libassist-sys | ASSIST | REBOUND |
|---|---|---|
| 1.2.x | [1.2.0](https://github.com/matthewholman/assist/releases/tag/v1.2.0) | [4.6.0](https://github.com/hannorein/rebound/releases/tag/4.6.0) (via librebound-sys 4.6.x) |

## License

GPL-3.0 — required by the vendored ASSIST and REBOUND sources. See [LICENSE](LICENSE) and [`vendor/assist/LICENSE`](vendor/assist/LICENSE).

## References

- Holman et al. 2023, "ASSIST: An Ephemeris-Quality Test Particle Integrator", [PSJ 4 69](https://doi.org/10.3847/PSJ/acc9a9) ([arXiv:2303.16246](https://arxiv.org/abs/2303.16246))
- Rein & Liu 2012, "REBOUND: An open-source multi-purpose N-body code for collisional dynamics", [A&A 537 A128](https://doi.org/10.1051/0004-6361/201118085)
- Rein & Spiegel 2015, "IAS15: a fast, adaptive, high-order integrator for gravitational dynamics", [MNRAS 446 1424](https://doi.org/10.1093/mnras/stu2164)

## Acknowledgments

This crate is a thin Rust wrapper. All credit for the underlying physics and integrator implementations belongs to the upstream projects:

- **ASSIST** — Matthew Holman and contributors. Source: <https://github.com/matthewholman/assist>. Vendored under [`vendor/assist/`](vendor/assist/) with original LICENSE preserved at [`vendor/assist/LICENSE`](vendor/assist/LICENSE).
- **REBOUND** — Hanno Rein and contributors. Source: <https://github.com/hannorein/rebound>. Used via the companion [`librebound-sys`](https://crates.io/crates/librebound-sys) crate, which vendors REBOUND with its LICENSE preserved.

If you use this crate in published work, please cite the ASSIST and REBOUND papers listed in [References](#references) — not this crate.

The Rust wrapper layer is developed by the [Asteroid Institute](https://asteroid.institute/), a program of the [B612 Foundation](https://b612foundation.org/).
