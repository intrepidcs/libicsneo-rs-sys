#![windows_subsystem = "console"]
use bindgen;
use cmake;
use path_clean::clean;
use std::env;
use std::path::PathBuf;
use which;

#[cfg(feature = "build_libicsneo")]
fn is_release_build() -> bool {
    let profile = std::env::var("PROFILE").unwrap();
    match profile.as_str() {
        "debug" => return false,
        "release" => return true,
        _ => return false,
    }
}

fn _checkout_libicsneo_repo(libicsneo_path: &PathBuf) {
    let output = std::process::Command::new("git")
        .args(["clone", "git@github.com:intrepidcs/libicsneo.git", "."])
        .current_dir(libicsneo_path)
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
        _checkout_libicsneo_repo(&libicsneo_path);
        panic!("Failed to clone libicsneo github repo!");
    }
}

fn _prepare_git_submodule(libicsneo_path: &PathBuf) {
    // This seems to not be needed when including this as a dependency? Why?
    // checkout the submodule if needed
    let output = std::process::Command::new("git")
        .args(["submodule", "update", "--init"])
        .current_dir(libicsneo_path)
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
        // If we are a dependency in another crate we aren't a git repo anymore.
        _checkout_libicsneo_repo(&libicsneo_path);
    }
}

#[cfg(feature = "build_libicsneo")]
fn build_libicsneo(libicsneo_path: &PathBuf) {
    use cmake::Config;

    // Check to make sure CMakeLists.txt exists
    if libicsneo_path.join("CMakeLists.txt").exists() {
        println!(
            "cargo:warning=Found CMakeLists.txt at: {}",
            libicsneo_path.display()
        );
    } else {
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
    let mut config = config.build_target("ALL_BUILD")
        .define("LIBICSNEO_BUILD_ICSNEOC_STATIC:BOOL", "ON")
        .define("LIBICSNEO_BUILD_EXAMPLES:BOOL", "OFF")
        .define("LIBICSNEO_BUILD_ICSNEOLEGACY:BOOL", "OFF")
        .profile(build_config_type); 
    // Lets use ninja if the feature is enabled
    let config = if cfg!(feature = "ninja") {
        // Check to make sure ninja exists on this platform's path
        match which::which("ninja") {
            Ok(_) => {},
            Err(e) => panic!("Failed to find ninja executable in system path. Is it installed? (Error: {:#?})", e),
        };
        config.generator("Ninja Multi-Config").build_target("all")
    } else {
        config
    };
    let dst = config.build();
    // output for lib path
    println!(
        "cargo:warning=icsneoc.lib search path: {:?}",
        dst.join(format!("build/{build_config_type}")).display()
    );
    // Configure the search path and lib name to link to for icsneoc-static and icsneocpp
    println!(
        "cargo:rustc-link-search=native={}",
        dst.join(format!("build/{build_config_type}")).display()
    );
    println!("cargo:rustc-link-lib=static=icsneoc-static");
    println!("cargo:rustc-link-lib=icsneocpp");
    // Configure the search path and lib name to link to for fatfs
    println!("cargo:warning=fatfs.lib search path: {}/build/third-party/fatfs/{build_config_type}", dst.display());
    // Configure the search path and lib name to link to
    println!(
        "cargo:rustc-link-search=native={}/build/third-party/fatfs/{build_config_type}", dst.display()
    );
    println!("cargo:rustc-link-lib=fatfs");
}

#[cfg(not(feature = "build_libicsneo"))]
fn build_libicsneo(_: &PathBuf) {}

fn generate_bindings(libicsneo_path: &PathBuf) {
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

fn main() {
    // Get the path of libicsneo
    let path = std::env::var("LIBICSNEO_PATH")
        .unwrap_or(format!("{}/src/libicsneo", env!("CARGO_MANIFEST_DIR")));
    let libicsneo_path = std::path::PathBuf::from(clean(&path));

    if std::env::var("DOCS_RS").is_err() {
        /*
        // We don't need to checkout the submodule if we are using a custom libicsneo path
        if std::env::var("LIBICSNEO_PATH").is_err() {
            _prepare_git_submodule(&libicsneo_path);
        }
        */
        build_libicsneo(&libicsneo_path);
    }
    // lets generate the bindings
    generate_bindings(&libicsneo_path);
}
