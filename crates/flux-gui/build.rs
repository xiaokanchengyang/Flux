//! Build script for flux-gui
//! Handles platform-specific build tasks like embedding icons

fn main() {
    // Only run icon embedding on Windows
    #[cfg(target_os = "windows")]
    {
        use std::env;
        use std::fs;
        use std::path::Path;

        // Only embed resource in release builds to avoid slowing down development
        if env::var("PROFILE").unwrap_or_default() == "release" {
            // Check if we have the icon files
            let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
            let icon_path = Path::new(&manifest_dir).join("assets").join("icon.ico");

            // If .ico doesn't exist, we'll need to convert from PNG
            if !icon_path.exists() {
                println!("cargo:warning=Windows icon (icon.ico) not found. Please convert assets/icon.png to .ico format");
                // For now, we'll skip embedding if the .ico doesn't exist
                return;
            }

            // Create a Windows resource file
            let rc_path = Path::new(&env::var("OUT_DIR").unwrap()).join("flux-gui.rc");
            let rc_content = format!(
                r#"1 ICON "{}""#,
                icon_path.to_str().unwrap().replace('\\', "\\\\")
            );
            fs::write(&rc_path, rc_content).expect("Failed to write resource file");

            // Use embed-resource to compile and link the resource
            embed_resource::compile(&rc_path, embed_resource::NONE);
        }
    }

    // Rerun build script if icon changes
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=assets/icon.png");
}
