use std::path::PathBuf;

use clap::Parser;

/// Transform Markdown into stunning PDFs with electric elegance.
#[derive(Debug, Parser)]
#[command(name = "silkprint", version, about)]
#[allow(clippy::struct_excessive_bools)]
pub struct Cli {
    /// Path to the Markdown file to render.
    pub input: Option<PathBuf>,

    /// Output path ("-" for stdout).
    #[arg(short, long, value_name = "PATH")]
    pub output: Option<String>,

    /// Theme name or path to .toml.
    #[arg(short, long, default_value = "silk-light")]
    pub theme: String,

    /// Paper size: a4, letter, a5, legal (case-insensitive).
    #[arg(short, long, default_value = "a4")]
    pub paper: String,

    /// List all available themes and exit.
    #[arg(long)]
    pub list_themes: bool,

    /// Validate input + theme without rendering.
    #[arg(long)]
    pub check: bool,

    /// Emit generated Typst markup instead of PDF.
    #[arg(long)]
    pub dump_typst: bool,

    /// Open the PDF in system viewer after rendering.
    #[arg(long)]
    pub open: bool,

    /// Force-enable table of contents.
    #[arg(long)]
    pub toc: bool,

    /// Force-disable table of contents.
    #[arg(long)]
    pub no_toc: bool,

    /// Suppress title page even if theme enables it.
    #[arg(long)]
    pub no_title_page: bool,

    /// Additional font search directory.
    #[arg(long, value_name = "DIR")]
    pub font_dir: Option<PathBuf>,

    /// Color output: auto, always, never.
    #[arg(long, default_value = "auto")]
    pub color: String,

    /// Increase verbosity (-v, -vv, -vvv).
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Suppress all output except errors.
    #[arg(short, long)]
    pub quiet: bool,
}

impl Cli {
    /// Validate flag combinations, returning errors for conflicts.
    pub fn validate(&self) -> Result<(), crate::error::SilkprintError> {
        if self.quiet && self.verbose > 0 {
            return Err(crate::error::SilkprintError::ConflictingOptions {
                details: "cannot combine --quiet and --verbose".to_string(),
            });
        }
        if self.check && self.open {
            return Err(crate::error::SilkprintError::ConflictingOptions {
                details: "--open requires rendering (incompatible with --check)".to_string(),
            });
        }
        if self.dump_typst && self.open {
            return Err(crate::error::SilkprintError::ConflictingOptions {
                details: "--open requires PDF output".to_string(),
            });
        }
        if self.toc && self.no_toc {
            return Err(crate::error::SilkprintError::ConflictingOptions {
                details: "cannot combine --toc and --no-toc".to_string(),
            });
        }
        if self.output.as_deref() == Some("-") && self.open {
            return Err(crate::error::SilkprintError::ConflictingOptions {
                details: "--open incompatible with stdout output".to_string(),
            });
        }
        Ok(())
    }
}
