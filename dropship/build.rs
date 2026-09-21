fn main() {
    // arabic ui font: thmanyah sans when the (non-redistributable) file is present,
    // otherwise the bundled IBM Plex Sans Arabic (OFL). copied into OUT_DIR so the
    // app can `include_bytes!` a fixed path either way.
    {
        let thmanyah = "assets/fonts/Thmanyah/thmanyahsans-Medium.otf";
        let fallback = "assets/fonts/fallback/IBMPlexSansArabic-Medium.ttf";
        let src = if std::path::Path::new(thmanyah).exists() {
            thmanyah
        } else {
            fallback
        };
        let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
        std::fs::copy(src, out.join("arabic-font.bin")).unwrap();
        println!("cargo:rerun-if-changed=assets/fonts/Thmanyah");
        println!("cargo:rerun-if-changed={fallback}");
    }

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    if target_os == "windows" {
        // VERSIONINFO يأخذ الإصدار من Cargo.toml حتى لا يتخلف عنه (كان يُكتب يدويًا)
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap().replace('\\', "/");
        let version = std::env::var("CARGO_PKG_VERSION").unwrap();
        let mut parts: Vec<String> = version.split('.').map(str::to_owned).collect();
        parts.resize(4, "0".to_owned());
        let rc = std::fs::read_to_string("assets/windows/dropship-manifest.rc")
            .unwrap()
            .replace("@VER_COMMA@", &parts.join(","))
            .replace("@VER@", &parts.join("."))
            .replace("@ASSETS@", &format!("{manifest_dir}/assets/windows"));
        let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("dropship.rc");
        std::fs::write(&out, rc).unwrap();
        println!("cargo:rerun-if-changed=assets/windows/dropship-manifest.rc");
        println!("cargo:rerun-if-changed=assets/windows/dropship.exe.manifest");
        embed_resource::compile(&out, embed_resource::NONE)
            .manifest_required()
            .unwrap();
    }
}
