fn main() {
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set_language(0x0411);
        if let Err(e) = res.compile() {
            eprintln!("winresource compile failed: {e} (icon optional)");
        }
    }
}
