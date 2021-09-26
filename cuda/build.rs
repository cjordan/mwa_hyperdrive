// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::env;
use std::path::PathBuf;

// This code is adapted from pkg-config-rs
// (https://github.com/rust-lang/pkg-config-rs).
#[allow(clippy::if_same_then_else, clippy::needless_bool)]
fn infer_static(name: &str) -> bool {
    if env::var(&format!("{}_STATIC", name.to_uppercase())).is_ok() {
        true
    } else if env::var(&format!("{}_DYNAMIC", name.to_uppercase())).is_ok() {
        false
    } else if env::var("PKG_CONFIG_ALL_STATIC").is_ok() {
        true
    } else if env::var("PKG_CONFIG_ALL_DYNAMIC").is_ok() {
        false
    } else {
        false
    }
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // Attempt to read HYPERDRIVE_CUDA_COMPUTE.
    println!("cargo:rerun-if-env-changed=HYPERDRIVE_CUDA_COMPUTE");
    let compute = match env::var("HYPERDRIVE_CUDA_COMPUTE") {
        Ok(c) => c,
        Err(e) => panic!(
            "Problem reading env. variable HYPERDRIVE_CUDA_COMPUTE ! {}",
            e
        ),
    };
    // Check that there's only two numeric characters.
    if compute.parse::<u16>().is_err() {
        panic!("HYPERDRIVE_CUDA_COMPUTE couldn't be parsed into a number!")
    }
    if compute.len() != 2 {
        panic!("HYPERDRIVE_CUDA_COMPUTE is not a two-digit number!")
    }

    // Compile all CUDA source files into a single library. Find .cu, .h and
    // .cuh files; if any of them change, tell cargo to recompile.
    let mut cuda_files = vec![];
    for entry in std::fs::read_dir("src_cuda").expect("src_cuda directory doesn't exist!") {
        let entry = entry.expect("Couldn't access file in src_cuda directory");
        let path = entry.path();
        // Skip this entry if it isn't a file.
        if !path.is_file() {
            continue;
        }

        match path.extension().and_then(|os_str| os_str.to_str()) {
            // Track this file if it's extension is .cu
            Some("cu") => {
                println!("cargo:rerun-if-changed={}", path.display());
                // Add this .cu file to be compiled later.
                cuda_files.push(path);
            }

            Some("h" | "cuh") => {
                println!("cargo:rerun-if-changed={}", path.display());
            }

            _ => (),
        }
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    // Add the FEE beam code!
    let mut fee_include = out_dir.clone();
    fee_include.push("fee.h");
    mwa_hyperbeam_cuda::fee::write_fee_cuda_file(&fee_include).unwrap();

    let mut cuda_target = cc::Build::new();
    cuda_target
        .cuda(true)
        .flag("-cudart=static")
        .flag("-gencode")
        // Using the specified HYPERDRIVE_CUDA_COMPUTE
        .flag(&format!("arch=compute_{c},code=sm_{c}", c = compute))
        .define(
            // The DEBUG env. variable is set by cargo. If running "cargo build
            // --release", DEBUG is "false", otherwise "true". C/C++/CUDA like
            // the compile option "NDEBUG" to be defined when using assert.h, so
            // if appropriate, define that here. We also define "DEBUG" so that
            // could be used.
            match env::var("DEBUG").as_deref() {
                Ok("false") => "NDEBUG",
                _ => "DEBUG",
            },
            None,
        )
        .include(&out_dir);

    // If we're told to, use single-precision floats. The default in the CUDA
    // code is to use double-precision.
    #[cfg(feature = "cuda-single")]
    cuda_target.define("SINGLE", None);

    cuda_target.files(&cuda_files).compile("hyperdrive_cu");

    // Link CUDA. If the library path manually specified, search there.
    if let Ok(lib_dir) = env::var("CUDA_LIB") {
        println!("cargo:rustc-link-search=native={}", lib_dir);
    }

    if infer_static("cuda") {
        // CUDA ships its static library as cudart_static.a, not cudart.a
        println!("cargo:rustc-link-lib=static=cudart_static");
    } else {
        println!("cargo:rustc-link-lib=cudart");
    }
}
