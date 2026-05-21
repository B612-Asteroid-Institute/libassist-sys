// build.rs — Compile vendored ASSIST C sources + ASSIST-side accessor helpers.
//
// ASSIST's C files `#include "rebound.h"`. The REBOUND include path is
// published by librebound-sys via `cargo:include=...`, which Cargo exposes
// here as the `DEP_REBOUND_INCLUDE` env var.

use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let assist_src = manifest_dir.join("vendor/assist/src");

    let rebound_include = env::var("DEP_REBOUND_INCLUDE")
        .expect("librebound-sys did not publish DEP_REBOUND_INCLUDE — check its build.rs prints cargo:include=");
    let rebound_include = PathBuf::from(rebound_include);

    // ASSIST C sources.
    let assist_sources: Vec<String> = ["assist", "forces", "spk", "ascii_ephem", "tools"]
        .iter()
        .map(|name| format!("{}/{}.c", assist_src.display(), name))
        .collect();

    // Build ASSIST as a static library.
    //
    // `-ffp-contract=off` matches REBOUND's upstream build for bit-for-bit
    // reproducibility (see librebound-sys/build.rs).
    // `-flto=thin` enables cross-TU inlining under clang; GCC silently
    // ignores via `flag_if_supported`.
    let mut assist_build = cc::Build::new();
    assist_build
        .include(&assist_src)
        .include(&rebound_include)
        .define("LIBASSIST", None)
        .define("_GNU_SOURCE", None)
        .flag_if_supported("-std=c99")
        .flag_if_supported("-ffp-contract=off")
        .flag_if_supported("-flto=thin")
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-sign-compare")
        .flag_if_supported("-Wno-unknown-pragmas")
        .opt_level(3)
        .pic(true);

    for src in &assist_sources {
        assist_build.file(src);
    }
    assist_build.compile("assist");

    // ASSIST-side helper functions for opaque assist_extras / assist_ephem access.
    cc::Build::new()
        .include(&assist_src)
        .include(&rebound_include)
        .file("src/helpers.c")
        .flag_if_supported("-std=c99")
        .flag_if_supported("-ffp-contract=off")
        .flag_if_supported("-flto=thin")
        .opt_level(3)
        .pic(true)
        .compile("libassist_sys_helpers");

    println!("cargo:rerun-if-changed=vendor/assist/src");
    println!("cargo:rerun-if-changed=src/helpers.c");

    // Expose ASSIST include path to consumers that might want to compile
    // additional C against ASSIST headers (rare; provided for symmetry).
    println!("cargo:include={}", assist_src.display());
}
