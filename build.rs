use cmake::Config;
use path_clean::{clean, PathClean};
use std::{env, path::PathBuf};

fn libicsneo_path() -> PathBuf {
    // Get the path of libicsneo
    let path = std::env::var("LIBICSNEO_PATH")
        .unwrap_or(format!("{}/src/libicsneo", env!("CARGO_MANIFEST_DIR")));
    let libicsneo_path = std::path::PathBuf::from(clean(&path));

    libicsneo_path
}

fn libicsneo_include_path() -> PathBuf {
    let path = libicsneo_path().join("include");
    path.clean()
}

fn libicsneo_header_path() -> PathBuf {
    let path = libicsneo_include_path().join("icsneo").join("icsneoc.h");
    path.clean()
}

fn is_release_build() -> bool {
    let profile = std::env::var("PROFILE").unwrap();
    match profile.as_str() {
        "debug" => return false,
        "release" => return true,
        _ => return false,
    }
}

fn build_libicsneo() {
    let libicsneo_path = libicsneo_path();
    // Check to make sure CMakeLists.txt exists
    if !libicsneo_path.join("CMakeLists.txt").exists() {
        panic!("CMakeLists.txt not found at {}", libicsneo_path.display());
    }
    let build_config_type = if cfg!(target_os = "windows") {
        // Rust runtime is linked with /MD on windows MSVC... MSVC takes Debug and forces /MDd
        // https://www.reddit.com/r/rust/comments/dvmzo2/cargo_external_c_library_windows_debugrelease_hell/
        if is_release_build() {
            "Release"
        } else {
            "RelWithDebInfo"
        }
    } else if is_release_build() {
        "Release"
    } else {
        "Debug"
    };
    // Run cmake on libicsneo
    let mut config = Config::new(libicsneo_path.clone());
    #[allow(unused_mut)]
    let mut config = config
        .build_target("ALL_BUILD")
        .define("LIBICSNEO_BUILD_ICSNEOC_STATIC:BOOL", "ON")
        .define("LIBICSNEO_BUILD_EXAMPLES:BOOL", "OFF")
        .define("LIBICSNEO_BUILD_ICSNEOLEGACY:BOOL", "OFF")
        .profile(build_config_type);
    // Lets use ninja if the feature is enabled
    let config = match which::which("ninja") {
        Ok(_) => config.generator("Ninja Multi-Config").build_target("all"),
        Err(_e) => config,
    };
    let dst = config.build();
    // output for lib path
    println!(
        "cargo:warning=icsneoc.lib search path: {:?}",
        dst.join(format!("build/{build_config_type}")).display()
    );
    // icsneo lib/dll linker search path
    println!(
        "cargo:rustc-link-search=native={}",
        dst.join(format!("build/{build_config_type}")).display()
    );
    // fatfs linker search path and addition
    println!(
        "cargo:rustc-link-search=native={}/build/third-party/fatfs/{build_config_type}",
        dst.display()
    );
    println!("cargo:rustc-link-lib=fatfs");
    // icsneocpp linker addition
    println!("cargo:rustc-link-lib=icsneocpp");

    /*
    println!("cargo:rustc-link-lib=static=icsneoc-static");
    println!("cargo:rustc-link-lib=icsneocpp");
    // Configure the search path and lib name to link to for fatfs
    println!("cargo:warning=fatfs.lib search path: {}/build/third-party/fatfs/{build_config_type}", dst.display());
    // Configure the search path and lib name to link to
    println!(
        "cargo:rustc-link-search=native={}/build/third-party/fatfs/{build_config_type}", dst.display()
    );
    println!("cargo:rustc-link-lib=fatfs");
    */
}

#[cfg(not(feature = "static"))]
fn setup_linker_libs() {
    println!("cargo:rustc-link-lib=dylib=icsneoc");
    match env::var("CARGO_CFG_TARGET_OS").unwrap().as_str() {
        "macos" => {
            println!("cargo:rustc-link-lib=framework=IOKit");
            println!("cargo:rustc-link-lib=framework=CoreFoundation");
        }
        "linux" | "windows" => {}
        target_os => panic!("Target OS not supported: {target_os}"),
    }
}

#[cfg(feature = "static")]
fn setup_linker_libs() {
    match env::var("CARGO_CFG_TARGET_OS").unwrap().as_str() {
        "windows" => {
            println!("cargo:rustc-link-lib=static=icsneoc-static");
        }
        "linux" => {
            println!("cargo:rustc-link-lib=static=icsneoc-static");
        }
        "macos" => {
            println!("cargo:rustc-link-lib=static=icsneoc-static");
            println!("cargo:rustc-link-lib=framework=IOKit");
            println!("cargo:rustc-link-lib=framework=CoreFoundation");
        }
        target_os => panic!("Target OS not supported: {target_os}"),
    }
}

fn generate_bindings() {
    let header = libicsneo_header_path();
    let bindings = bindgen::Builder::default()
        .header(header.to_str().unwrap())
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: false,
        })
        .clang_args(&[format!("-I{}", libicsneo_include_path().display()).as_str()])
        .allowlist_function("icsneo_.*")
        .allowlist_type("neodevice_t")
        .allowlist_type("neonetid_t")
        //.allowlist_type("neomessage_.*")
        .allowlist_type("neoversion_t")
        .allowlist_type("neoevent_t")
        //.formatter(bindgen::Formatter::Rustfmt)
        .derive_default(true)
        .derive_debug(true)
        //.derive_partialeq(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        //.clang_args(clang_args())
        .generate()
        .expect("Unable to generate bindings");
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    println!("cargo:warning=out_path: {:?}", out_path.display());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings");

    let out_path = std::path::PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

fn main() {
    let header = libicsneo_header_path();
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", header.to_str().unwrap());
    println!("cargo:rerun-if-env-changed=LIBMSVC_PATH");

    generate_bindings();
    build_libicsneo();
    setup_linker_libs();
}
