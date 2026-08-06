fn main() {
    #[cfg(windows)]
    {
        // Only embed icon when a real .ico exists. Placeholder text files are ignored.
        let icon = std::path::Path::new("assets/icon.ico");
        if icon.is_file() {
            if let Ok(meta) = std::fs::metadata(icon) {
                // Real ICO files are larger than a few dozen bytes
                if meta.len() > 64 {
                    let mut res = winresource::WindowsResource::new();
                    res.set_icon("assets/icon.ico");
                    res.set_language(0x0411);
                    if let Err(e) = res.compile() {
                        eprintln!("winresource compile failed: {e} (continuing without icon)");
                    }
                }
            }
        }
    }
}
