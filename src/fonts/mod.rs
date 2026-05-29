// ── Native: embed all fonts at compile time ─────────────────────────
#[cfg(not(target_arch = "wasm32"))]
mod native {
    use rust_embed::RustEmbed;

    /// Core bundled fonts for text, code, and math.
    #[derive(RustEmbed)]
    #[folder = "fonts/core/"]
    struct CoreFonts;

    /// Emoji font (Noto Color Emoji) — only bundled for native CLI.
    #[derive(RustEmbed)]
    #[folder = "fonts/emoji/"]
    struct EmojiFonts;

    pub fn load_bundled_fonts() -> Vec<Vec<u8>> {
        let mut fonts = Vec::new();
        for filename in CoreFonts::iter() {
            if let Some(file) = CoreFonts::get(&filename) {
                fonts.push(file.data.to_vec());
            }
        }
        for filename in EmojiFonts::iter() {
            if let Some(file) = EmojiFonts::get(&filename) {
                fonts.push(file.data.to_vec());
            }
        }
        fonts
    }
}

// ── WASM: fonts loaded externally via register_font() ───────────────
#[cfg(target_arch = "wasm32")]
mod wasm {
    use std::cell::RefCell;

    thread_local! {
        static EXTERNAL_FONTS: RefCell<Vec<Vec<u8>>> = const { RefCell::new(Vec::new()) };
    }

    /// Register a font loaded from JS. Called once per font file before rendering.
    pub fn add_external_font(data: Vec<u8>) {
        EXTERNAL_FONTS.with(|fonts| {
            let mut fonts = fonts.borrow_mut();
            if fonts.iter().all(|existing| existing != &data) {
                fonts.push(data);
            }
        });
    }

    /// Clear all externally registered fonts.
    pub fn clear_external_fonts() {
        EXTERNAL_FONTS.with(|fonts| fonts.borrow_mut().clear());
    }

    /// Clone all externally registered fonts into the Typst font pool.
    pub fn load_bundled_fonts() -> Vec<Vec<u8>> {
        EXTERNAL_FONTS.with(|fonts| {
            let stored = fonts.borrow();
            stored.clone()
        })
    }
}

// ── Re-exports ──────────────────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
pub use native::load_bundled_fonts;

#[cfg(target_arch = "wasm32")]
pub use wasm::load_bundled_fonts;

/// Register a font loaded externally (WASM only — no-op on native).
#[cfg(target_arch = "wasm32")]
pub use wasm::add_external_font;

/// Clear previously registered fonts (WASM only — no-op on native).
#[cfg(target_arch = "wasm32")]
pub use wasm::clear_external_fonts;

/// No-op on native — fonts are embedded at compile time.
#[cfg(not(target_arch = "wasm32"))]
pub fn add_external_font(_data: Vec<u8>) {}

/// No-op on native — fonts are embedded at compile time.
#[cfg(not(target_arch = "wasm32"))]
pub fn clear_external_fonts() {}
