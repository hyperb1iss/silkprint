//! Graphics-capability probe for the terminal reader.
//!
#![allow(clippy::print_stdout, clippy::doc_markdown)]
//!
//! Run inside the terminal you actually use the reader in:
//!     cargo run --example picker_probe --features terminal
//!
//! It queries the terminal the same way `silkprint read` does and prints what
//! it detected, so we can tell whether inline images / rasterized headings /
//! mermaid have a working graphics protocol to draw on.

#[cfg(feature = "terminal")]
fn main() {
    use ratatui_image::picker::Picker;
    match Picker::from_query_stdio() {
        Ok(picker) => {
            let fs = picker.font_size();
            println!("picker: OK");
            println!("  protocol  : {:?}", picker.protocol_type());
            println!("  font cell : {}x{} px", fs.width, fs.height);
            println!("  caps      : {:?}", picker.capabilities());
        }
        Err(e) => {
            println!("picker: ERROR — falling back to text-only");
            println!("  error: {e}");
        }
    }
}

#[cfg(not(feature = "terminal"))]
fn main() {
    println!("build with --features terminal");
}
