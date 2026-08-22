fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=assets/app.ico");
        println!("cargo:rerun-if-changed=assets/app-dev.ico");

        let mut res = winres::WindowsResource::new();
        let profile = std::env::var("PROFILE").unwrap_or_default();
        let (icon, display_name) = if profile == "release" {
            ("assets/app.ico", "Zapret Interactive")
        } else {
            ("assets/app-dev.ico", "Zapret Interactive (Dev)")
        };
        res.set_icon(icon)
            .set("ProductName", display_name)
            .set("FileDescription", display_name)
            .set("InternalName", display_name)
            .set("OriginalFilename", "Zapret Interactive.exe")
            .set("CompanyName", "Noktomezo")
            .set("LegalCopyright", "Copyright © 2026 Noktomezo");
        res.compile()?;
    }
    Ok(())
}
