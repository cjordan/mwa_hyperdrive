// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::env;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Build bindings to C/C++ functions.
    let bindings = bindgen::Builder::default()
        .header("src_cuda/vis_gen.h")
        // Invalidate the built crate whenever any of the included header files
        // changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    // Attempt to read HYPERDRIVE_CUDA_COMPUTE.
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

    // Compile all CUDA source files into a single library.
    let mut cuda_files = vec![];
    for entry in std::fs::read_dir("src_cuda").expect("src_cuda directory doesn't exist!") {
        let entry = entry.expect("Couldn't access file in src_cuda directory");
        // let path = entry.path().to_path_buf();
        let path = entry.path();
        // Skip this entry if it isn't a file.
        if !path.is_file() {
            continue;
        }

        // Continue if this file's extension is .cu
        if let Some("cu") = path.extension().map(|os_str| os_str.to_str().unwrap()) {
            // Add this .cu file to be compiled later.
            cuda_files.push(path);
        }
    }

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
            // if appropriate, define that here. "DEBUG" is also defined, and
            // could also be used.
            if env::var("DEBUG").unwrap() == "false" {
                "NDEBUG"
            } else {
                "DEBUG"
            },
            None,
        );
    for f in cuda_files {
        println!("cargo:rerun-if-changed={}", f.display());
        cuda_target.file(f);
    }
    cuda_target.compile("hyperdrive_cu");

    // Use the following search paths when linking.
    // CUDA could be installed in a couple of places, and use "lib" or "lib64";
    // specify all combinations.
    for path in &["/usr/local/cuda", "/opt/cuda"] {
        for lib_path in &["lib", "lib64"] {
            println!("cargo:rustc-link-search=native={}/{}", path, lib_path);
        }
    }

    // Link with the dynamic cudart library
    println!("cargo:rustc-link-lib=cudart");
}