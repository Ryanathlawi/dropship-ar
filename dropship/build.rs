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
        embed_resource::compile("assets/windows/dropship-manifest.rc", embed_resource::NONE)
            .manifest_required()
            .unwrap();
    }
}
