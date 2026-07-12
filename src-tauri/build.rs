fn main() {
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rerun-if-changed=macos/SessionIslandBridge.h");
        build_session_island_panel();
        build_capture_helpers();

        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=SwiftUI");
    }

    tauri_build::build()
}

#[cfg(target_os = "macos")]
fn build_capture_helpers() {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    const HELPERS: [(&str, &str, &[&str]); 6] = [
        (
            "capture_events",
            "scripts/capture_events.swift",
            &["ApplicationServices", "AppKit"],
        ),
        (
            "window_snapshot",
            "scripts/window_snapshot.swift",
            &["AppKit", "CoreGraphics", "ApplicationServices"],
        ),
        (
            "accessibility_snapshot",
            "scripts/accessibility_snapshot.swift",
            &["ApplicationServices", "AppKit"],
        ),
        (
            "vision_ocr",
            "scripts/vision_ocr.swift",
            &["Vision", "AppKit"],
        ),
        (
            "sck_screenshot",
            "scripts/sck_screenshot.swift",
            &["AppKit", "CoreGraphics", "ScreenCaptureKit"],
        ),
        ("image_mask", "scripts/image_mask.swift", &["AppKit"]),
    ];

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR is required"));
    let helper_dir = out_dir.join("swift-helpers");
    let module_cache_path = out_dir.join("swift-helper-module-cache");
    std::fs::create_dir_all(&helper_dir).expect("failed to create Swift helper output directory");
    std::fs::create_dir_all(&module_cache_path)
        .expect("failed to create Swift helper module cache directory");

    let sdk_path = Command::new("xcrun")
        .args(["--sdk", "macosx", "--show-sdk-path"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .expect("failed to resolve the macOS SDK path with xcrun");
    let target_arch =
        std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "aarch64".to_string());
    let swift_target = if target_arch == "x86_64" {
        "x86_64-apple-macos13.0"
    } else {
        "arm64-apple-macos13.0"
    };
    for (name, source, frameworks) in HELPERS {
        println!("cargo:rerun-if-changed={source}");
        let out_helper = helper_dir.join(name);
        compile_swift_executable(
            name,
            Path::new(source),
            &out_helper,
            frameworks,
            &sdk_path,
            swift_target,
            &module_cache_path,
        );
    }
}

#[cfg(target_os = "macos")]
fn compile_swift_executable(
    name: &str,
    source: &std::path::Path,
    output_path: &std::path::Path,
    frameworks: &[&str],
    sdk_path: &str,
    swift_target: &str,
    module_cache_path: &std::path::Path,
) {
    use std::process::Command;

    let mut command = Command::new("swiftc");
    command
        .args([
            "-O",
            "-swift-version",
            "5",
            "-sdk",
            sdk_path,
            "-target",
            swift_target,
        ])
        .arg("-module-cache-path")
        .arg(module_cache_path)
        .arg(source);
    for framework in frameworks {
        command.arg("-framework").arg(framework);
    }
    let result = command
        .arg("-o")
        .arg(output_path)
        .output()
        .unwrap_or_else(|error| panic!("failed to run swiftc for {name}: {error}"));
    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        panic!(
            "swiftc failed for {name}: {}",
            stderr.chars().take(4000).collect::<String>()
        );
    }
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
