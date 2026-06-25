fn main() {
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rerun-if-changed=macos/SessionIslandBridge.h");
        build_session_island_panel();

        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=SwiftUI");
    }

    tauri_build::build()
}

#[cfg(target_os = "macos")]
fn build_session_island_panel() {
    use std::path::PathBuf;
    use std::process::Command;

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let swift_src = PathBuf::from("macos/SessionIslandPanel.swift");
    let lib_path = out_dir.join("libsession_island_panel.a");
    let module_cache_path = out_dir.join("swift-module-cache");
    std::fs::create_dir_all(&module_cache_path)
        .expect("failed to create Swift module cache directory");

    println!("cargo:rerun-if-changed=macos/SessionIslandPanel.swift");

    let sdk_path = Command::new("xcrun")
        .args(["--sdk", "macosx", "--show-sdk-path"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .unwrap_or_default();
    let sdk_path = sdk_path.trim().to_string();

    let target_arch =
        std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "aarch64".to_string());
    let swift_target = if target_arch == "x86_64" {
        "x86_64-apple-macos13.0"
    } else {
        "arm64-apple-macos13.0"
    };

    let output = Command::new("swiftc")
        .args([
            "-emit-library",
            "-static",
            "-module-name",
            "SessionIslandPanel",
            "-swift-version",
            "5",
            "-sdk",
            &sdk_path,
            "-target",
            swift_target,
            "-module-cache-path",
        ])
        .arg(&module_cache_path)
        .args(["-O", "-whole-module-optimization", "-o"])
        .arg(&lib_path)
        .arg(&swift_src)
        .output()
        .expect("failed to run swiftc for SessionIslandPanel.swift");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "swiftc failed for SessionIslandPanel.swift: {}",
            stderr.chars().take(2000).collect::<String>()
        );
    }

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=session_island_panel");
}
