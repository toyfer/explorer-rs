fn main() {
    #[cfg(windows)]
    {
        if let Some(icon_path) = generate_icon_if_needed() {
            let mut res = winresource::WindowsResource::new();
            res.set_icon(&icon_path);
            res.set_language(0x0411);
            // Explorer++ style metadata
            res.set("FileDescription", "explorer-rs — high-performance file manager");
            res.set("ProductName", "explorer-rs");
            if let Err(e) = res.compile() {
                eprintln!("cargo:warning=winresource: {e} (continuing without icon)");
            } else {
                println!("cargo:warning=embedded icon {}", icon_path);
            }
        } else {
            println!("cargo:warning=skipping icon embed (generation failed)");
        }
    }
}

#[cfg(windows)]
fn generate_icon_if_needed() -> Option<String> {
    // Prefer a real assets/icon.ico if it exists and is valid (>1KB)
    let asset = std::path::Path::new("assets/icon.ico");
    if asset.is_file() {
        if let Ok(m) = std::fs::metadata(asset) {
            if m.len() > 1024 {
                return Some("assets/icon.ico".to_string());
            }
        }
    }
    // Otherwise generate a minimal 32x32 ICO in OUT_DIR
    let out_dir = std::env::var("OUT_DIR").ok()?;
    let out_path = std::path::Path::new(&out_dir).join("generated_icon.ico");
    if let Err(e) = write_minimal_ico(&out_path) {
        eprintln!("cargo:warning=icon generation failed: {e}");
        return None;
    }
    Some(out_path.to_string_lossy().to_string())
}

#[cfg(windows)]
fn write_minimal_ico(path: &std::path::Path) -> std::io::Result<()> {
    if path.is_file() {
        // already generated, reuse
        return Ok(());
    }
    let mut ico = Vec::new();
    // ICO header: reserved 0, type 1 (icon), count 1
    ico.extend(&[0u8, 0, 1, 0, 1, 0]);
    let width: u8 = 32;
    let height: u8 = 32;
    let planes: u16 = 1;
    let bpp: u16 = 32;
    let bmp_header: u32 = 40;
    let pixel_bytes: u32 = 32 * 32 * 4;
    let mask_bytes: u32 = 32 * 32 / 8; // AND mask, all zero = opaque
    let image_size: u32 = bmp_header + pixel_bytes + mask_bytes;
    let offset: u32 = 6 + 16;
    ico.push(width);
    ico.push(height);
    ico.push(0); // colors
    ico.push(0); // reserved
    ico.extend(&planes.to_le_bytes());
    ico.extend(&bpp.to_le_bytes());
    ico.extend(&image_size.to_le_bytes());
    ico.extend(&offset.to_le_bytes());
    // BITMAPINFOHEADER (40 bytes)
    ico.extend(&40u32.to_le_bytes()); // header size
    ico.extend(&32i32.to_le_bytes()); // width
    ico.extend(&64i32.to_le_bytes()); // height *2 (xor + and)
    ico.extend(&1u16.to_le_bytes()); // planes
    ico.extend(&32u16.to_le_bytes()); // bpp
    ico.extend(&0u32.to_le_bytes()); // compression BI_RGB
    ico.extend(&0u32.to_le_bytes()); // image size (0 for BI_RGB)
    ico.extend(&0i32.to_le_bytes()); // X ppm
    ico.extend(&0i32.to_le_bytes()); // Y ppm
    ico.extend(&0u32.to_le_bytes()); // colors used
    ico.extend(&0u32.to_le_bytes()); // important colors
    // Pixel data bottom-up, BGRA
    for y in (0..32).rev() {
        for x in 0..32 {
            let (r, g, b, a) = ico_pixel(x, y);
            ico.push(b);
            ico.push(g);
            ico.push(r);
            ico.push(a);
        }
    }
    // AND mask (128 bytes zeros = opaque)
    ico.extend(vec![0u8; mask_bytes as usize]);
    std::fs::write(path, ico)
}

#[cfg(windows)]
fn ico_pixel(x: u32, y: u32) -> (u8, u8, u8, u8) {
    // Explorer++ inspired: dark title bar + blue folder body + white lines
    // y 0 is bottom in ICO, but we generate with y from 0..31 bottom-up in caller reversal,
    // so here y is actual row with 0=bottom. Convert to top-down for design:
    let ty = 31 - y; // top-down y
    // Background transparent outside rounded folder shape
    // Simple folder shape: top tab + body
    let is_tab = ty < 6 && x >= 2 && x < 14;
    let is_body = ty >= 6 && ty < 28 && x >= 2 && x < 30;
    let is_border = (is_tab || is_body) && (x == 2 || x == 29 || ty == 6 || ty == 27 || (is_tab && ty == 2));
    if is_tab || is_body {
        if is_border {
            return (30, 58, 138, 255); // darker border #1e3a8a
        }
        // folder fill #3b82f6 -> #2563eb gradient by y
        let shade = if ty < 12 { (59, 130, 246) } else { (37, 99, 235) };
        // white file lines
        if ty >= 12 && ty <= 13 && x >= 6 && x < 26 { return (255, 255, 255, 230); }
        if ty >= 16 && ty <= 17 && x >= 6 && x < 26 { return (255, 255, 255, 230); }
        if ty >= 20 && ty <= 21 && x >= 6 && x < 20 { return (255, 255, 255, 230); }
        return (shade.0, shade.1, shade.2, 255);
    }
    (0, 0, 0, 0) // transparent
}
