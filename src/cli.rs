use std::path::PathBuf;

use clap::Parser;

/// SilkCircuit-themed help template for clap.
///
/// Uses ANSI truecolor sequences for the Electric Purple / Neon Cyan palette.
const HELP_TEMPLATE: &str = "\
\x1b[38;2;225;53;255m\x1b[1m\u{1f48e} {name}\x1b[0m \x1b[38;2;255;106;193m{version}\x1b[0m
\x1b[2m{about}\x1b[0m

\x1b[38;2;128;255;234m\x1b[1mUsage:\x1b[0m {usage}

\x1b[38;2;128;255;234m\x1b[1mArguments:\x1b[0m
{positionals}

\x1b[38;2;128;255;234m\x1b[1mOptions:\x1b[0m
{options}";

/// Transform Markdown into stunning PDFs with electric elegance.
#[derive(Debug, Parser)]
#[command(
    name = "silkprint",
    version,
    about = "Transform Markdown into stunning PDFs with electric elegance",
    help_template = HELP_TEMPLATE,
)]
#[allow(clippy::struct_excessive_bools)]
pub struct Cli {
    /// Path to the Markdown file to render (optional with --list-themes).
    pub input: Option<PathBuf>,

    /// Output path ("-" for stdout) [default: <input-stem>.pdf].
    #[arg(short, long, value_name = "PATH")]
    pub output: Option<String>,

    /// Theme name or path to .toml file.
    #[arg(short, long, default_value = "silkcircuit-dawn", value_name = "NAME")]
    pub theme: String,

    /// Paper size: a4, letter, a5, legal (case-insensitive).
    #[arg(short, long, default_value = "a4", value_name = "SIZE")]
    pub paper: String,

    /// List all available themes and exit.
    #[arg(long)]
    pub list_themes: bool,

    /// Validate input + theme without rendering (exit code only).
    #[arg(long)]
    pub check: bool,

    /// Emit generated Typst markup instead of PDF.
    #[arg(long)]
    pub dump_typst: bool,

    /// Open the PDF in system viewer after rendering.
    #[arg(long)]
    pub open: bool,

    /// Force-enable table of contents (overrides front matter).
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
    #[arg(long, default_value = "auto", value_name = "WHEN")]
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

    /// Resolve the TOC override from `--toc` / `--no-toc` flags.
    pub fn toc_override(&self) -> Option<bool> {
        if self.toc {
            Some(true)
        } else if self.no_toc {
            Some(false)
        } else {
            None
        }
    }

    /// Resolve the title page override from `--no-title-page`.
    pub fn title_page_override(&self) -> Option<bool> {
        if self.no_title_page {
            Some(false)
        } else {
            None
        }
    }

    /// Determine the output path for PDF mode.
    ///
    /// Returns `None` for stdout (`-o -`), otherwise resolves default
    /// from the input filename stem.
    pub fn resolve_output_path(&self, input: &std::path::Path) -> Option<PathBuf> {
        match self.output.as_deref() {
            Some("-") => None,
            Some(path) => Some(PathBuf::from(path)),
            None => {
                let stem = input.file_stem().unwrap_or_default();
                let mut out = PathBuf::from(stem);
                out.set_extension("pdf");
                Some(out)
            }
        }
    }
}
