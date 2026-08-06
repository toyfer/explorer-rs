fn main() {
    // Never fail the build because of optional Windows resources.
    #[cfg(windows)]
    {
        let icon = std::path::Path::new("assets/icon.ico");
        // Only try embed when a real ICO exists (placeholder text files are skipped).
        let ok = icon.is_file()
            && std::fs::metadata(icon)
                .map(|m| m.len() > 256)
                .unwrap_or(false);
        if ok {
            let mut res = winresource::WindowsResource::new();
            res.set_icon("assets/icon.ico");
            res.set_language(0x0411);
            if let Err(e) = res.compile() {
                eprintln("cargo:warning=winresource: {e} (continuing without icon)");
            }
        } else {
            println!("cargo:warning=skipping icon embed (no valid assets/icon.ico)");
        }
    }
}
