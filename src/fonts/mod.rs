use rust_embed::RustEmbed;

/// Bundled fonts embedded at compile time with compression.
///
/// Ships Inter, Source Serif 4, and `JetBrains` Mono.
#[derive(RustEmbed)]
#[folder = "fonts/"]
pub struct BundledFonts;

/// Load all bundled font data.
pub fn load_bundled_fonts() -> Vec<Vec<u8>> {
    let mut fonts = Vec::new();
    for filename in BundledFonts::iter() {
        if let Some(file) = BundledFonts::get(&filename) {
            fonts.push(file.data.to_vec());
        }
    }
    fonts
}
