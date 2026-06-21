fn main() {
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rerun-if-changed=macos/SessionIslandBridge.h");
        println!("cargo:rerun-if-changed=macos/SessionIslandPanel.mm");

        cc::Build::new()
            .cpp(true)
            .file("macos/SessionIslandPanel.mm")
            .flag("-std=c++17")
            .flag("-fobjc-arc")
            .compile("session_island_panel");

        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=QuartzCore");
    }

    tauri_build::build()
}
