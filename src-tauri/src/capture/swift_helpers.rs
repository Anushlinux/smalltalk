use std::path::{Path, PathBuf};

const HELPER_NAMES: &[&str] = &[
    "capture_events",
    "window_snapshot",
    "accessibility_snapshot",
    "vision_ocr",
    "sck_screenshot",
    "image_mask",
];

pub(super) fn available(name: &str) -> bool {
    resolve(name).is_ok()
}

pub(super) fn resolve(name: &str) -> Result<PathBuf, String> {
    if !HELPER_NAMES.contains(&name) {
        return Err(format!("unknown packaged Swift helper: {name}"));
    }
    if !cfg!(target_os = "macos") {
        return Err("packaged Swift helpers are only available on macOS".to_string());
    }

    // Tauri places externalBin sidecars beside the main executable in
    // Contents/MacOS and signs them as nested code with the app bundle.
    if let Ok(executable) = std::env::current_exe() {
        if let Some(directory) = executable.parent() {
            if is_bundle_macos_directory(directory) {
                let sidecar = directory.join(name);
                if sidecar.is_file() {
                    return Ok(sidecar);
                }
                return Err(format!(
                    "signed app bundle is missing required Swift sidecar: {name}"
                ));
            }
        }
    }

    #[cfg(debug_assertions)]
    {
        // Cargo installs development helpers into one profile-stable directory.
        // Packaged builds never fall back to a repository or build directory.
        if let Some(directory) = option_env!("SMALLTALK_DEV_HELPER_DIR") {
            let stable_helper = Path::new(directory).join(name);
            if stable_helper.is_file() {
                return Ok(stable_helper);
            }
        }

        // This fallback supports unusual Cargo test layouts.
        let build_helper = Path::new(env!("OUT_DIR")).join("swift-helpers").join(name);
        if build_helper.is_file() {
            return Ok(build_helper);
        }
    }

    Err(format!("packaged Swift helper is missing: {name}"))
}

fn is_bundle_macos_directory(directory: &Path) -> bool {
    directory.file_name().and_then(|name| name.to_str()) == Some("MacOS")
        && directory
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            == Some("Contents")
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn every_runtime_helper_is_precompiled_as_a_macho() {
        for name in HELPER_NAMES {
            let path = resolve(name).expect("helper must be precompiled");
            let bytes = fs::read(path).expect("helper bytes");
            assert!(bytes.len() > 4);
            assert_eq!(&bytes[..4], &[0xcf, 0xfa, 0xed, 0xfe]);
        }
    }

    #[test]
    fn development_helpers_resolve_from_the_profile_stable_directory() {
        let stable_directory = Path::new(
            option_env!("SMALLTALK_DEV_HELPER_DIR")
                .expect("macOS build must publish a stable development helper directory"),
        );
        for name in HELPER_NAMES {
            assert_eq!(resolve(name).unwrap(), stable_directory.join(name));
        }
    }

    #[test]
    fn unknown_helper_names_are_rejected() {
        assert!(resolve("not-a-helper").is_err());
    }

    #[test]
    fn bundle_macos_directory_is_detected_without_accepting_build_paths() {
        assert!(is_bundle_macos_directory(Path::new(
            "/Applications/smalltalk.app/Contents/MacOS"
        )));
        assert!(!is_bundle_macos_directory(Path::new(
            "/workspace/src-tauri/target/debug"
        )));
    }
}
