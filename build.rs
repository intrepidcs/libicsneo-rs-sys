#![windows_subsystem = "console"]
extern crate bindgen;
extern crate cmake;
extern crate path_clean;

use std::env;
use std::path::PathBuf;
use cmake::Config;
use path_clean::clean;


fn main() {
    // Get the path of libicsneo
    let path = format!("{}/src/libicsneo", env!("CARGO_MANIFEST_DIR"));
    let libicsneo_path = std::path::PathBuf::from(clean(&path));
    // checkout the submodule if needed
    let output = std::process::Command::new("git")
        .args([
            "submodule",
            "update",
            "--init",
        ])
        .current_dir(path)
        .output()
        .expect("Failed to fetch git submodules!");
    // Make sure git was successful!
    if !output.status.success() {
        println!("cargo:warning=git return code: {}", output.status);
        let stdout = std::str::from_utf8(&output.stdout).unwrap();
        for line in stdout.split("\n") {
            println!("cargo:warning=git stdout: {}", line);
        }
        let stderr = std::str::from_utf8(&output.stderr).unwrap();
        for line in stderr.split("\n") {
            println!("cargo:warning=git stderr: {}", line);
        }
        panic!("Failed to fetch git submodules!");
    }   
    // Check to make sure CMakeLists.txt exists
    if libicsneo_path.join("CMakeLists.txt").exists() {
        println!("cargo:warning=Found CMakeLists.txt at: {}", libicsneo_path.display());
    } else {
        panic!("CMakeLists.txt not found at {}", libicsneo_path.display());
    }
    // Run cmake on libicsneo
    let dst = Config::new(libicsneo_path.clone())
        .build_target("icsneoc")
        .define("LIBICSNEO_BUILD_EXAMPLES", "OFF")
        .define("LIBICSNEO_BUILD_ICSNEOC_STATIC", "ON")
        .define("LIBICSNEO_BUILD_ICSNEOLEGACY", "OFF")
        .build();
    // Debug output for lib path
    println!("cargo:warning=icsneoc.lib search path: {:?}", dst.join("build/Debug").display());
    println!("cargo:warning=icsneoc.lib search path: {:?}", dst.join("build/Release").display());
    // Configure the search path and lib name to link to
    println!("cargo:rustc-link-search=native={}", dst.join("build/Debug").display());
    println!("cargo:rustc-link-search=native={}", dst.join("build/Release").display());
    println!("cargo:rustc-link-lib=static=icsneoc");
    // lets generate the bindings
    let include_path = libicsneo_path.join("include");
    println!("cargo:warning=icsneo include path: {:?}", include_path);
    println!("cargo:warning={}", include_path.display());
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg(format!("-I{}", include_path.display()))
        .generate()
        .expect("Unable to generate bindings");
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    println!("cargo:warning=out_path: {:?}", out_path.display());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings");
}
