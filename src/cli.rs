use std::path::PathBuf;

use clap::Parser;

/// SilkCircuit-themed help template for clap.
///
/// Uses ANSI truecolor sequences for the Electric Purple / Neon Cyan palette.
const HELP_TEMPLATE: &str = "\
\x1b[38;2;225;53;255m\x1b[1m\u{1f48e} {name}\x1b[0m \x1b[38;2;255;106;193m{version}\x1b[0m
\x1b[2m{about}\x1b[0m

\x1b[38;2;128;255;234m\x1b[1mUsage:\x1b[0m {usage}

\x1b[38;2;128;255;234m\x1b[1mCommands:\x1b[0m
{subcommands}

\x1b[38;2;128;255;234m\x1b[1mArguments:\x1b[0m
{positionals}

\x1b[38;2;128;255;234m\x1b[1mOptions:\x1b[0m
{options}";

/// Read Markdown in your terminal, or render it to a stunning PDF.
#[derive(Debug, Parser)]
#[command(
    name = "silkprint",
    version,
    about = "Read Markdown in your terminal, or render it to a stunning PDF",
    help_template = HELP_TEMPLATE,
)]
#[allow(clippy::struct_excessive_bools)]
pub struct Cli {
    /// Markdown file. Opens in the terminal reader by default; renders a PDF
    /// with `-o`/`--output` or the `pdf` subcommand.
    pub input: Option<PathBuf>,

    /// Theme name or path to .toml file.
    #[arg(
        short,
        long,
        global = true,
        default_value = "silkcircuit-dawn",
        value_name = "NAME"
    )]
    pub theme: String,

    /// Paper size: a4, letter, a5, legal (case-insensitive).
    #[arg(short, long, global = true, default_value = "a4", value_name = "SIZE")]
    pub paper: String,

    /// List all available themes and exit.
    #[arg(long)]
    pub list_themes: bool,

    /// Output path ("-" for stdout). Implies PDF output [default: <input-stem>.pdf].
    #[arg(short, long, global = true, value_name = "PATH")]
    pub output: Option<String>,

    /// Validate input + theme without rendering (exit code only). Implies PDF.
    #[arg(long, global = true)]
    pub check: bool,

    /// Emit generated Typst markup instead of PDF. Implies PDF output.
    #[arg(long, global = true)]
    pub dump_typst: bool,

    /// Emit HTML instead of PDF. Implies export output.
    #[arg(long, global = true)]
    pub dump_html: bool,

    /// Check local and remote links during validation.
    #[arg(long, global = true)]
    pub validate_links: bool,

    /// Open the PDF in system viewer after rendering. Implies PDF output.
    #[arg(long, global = true)]
    pub open: bool,

    /// Force-enable table of contents (overrides front matter).
    #[arg(long, global = true)]
    pub toc: bool,

    /// Force-disable table of contents.
    #[arg(long, global = true)]
    pub no_toc: bool,

    /// Suppress title page even if theme enables it.
    #[arg(long, global = true)]
    pub no_title_page: bool,

    /// Additional font search directory.
    #[arg(long, global = true, value_name = "DIR")]
    pub font_dir: Option<PathBuf>,

    /// Force one-shot styled output even in an interactive terminal.
    #[cfg(feature = "terminal")]
    #[arg(long, global = true)]
    pub plain: bool,

    /// Disable auto-paging for long one-shot reader output.
    #[cfg(feature = "terminal")]
    #[arg(long, global = true)]
    pub no_pager: bool,

    /// Glyph set: nerdfont (default), unicode, ascii.
    #[cfg(feature = "terminal")]
    #[arg(long, global = true, value_name = "MODE")]
    pub glyphs: Option<String>,

    /// Disable inline image rendering in the reader.
    #[cfg(feature = "terminal")]
    #[arg(long, global = true)]
    pub no_images: bool,

    /// Wrap one-shot output to this many columns (default: terminal width).
    #[cfg(feature = "terminal")]
    #[arg(long, global = true, value_name = "COLS")]
    pub width: Option<u16>,

    /// Color output: auto, always, never.
    #[arg(long, global = true, default_value = "auto", value_name = "WHEN")]
    pub color: String,

    /// Increase verbosity (-v, -vv, -vvv).
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Suppress all output except errors.
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Explicit mode subcommand. Absent → read the input in the terminal
    /// (or one-shot when piped), unless a PDF flag forces rendering.
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Explicit mode subcommands. The bare form (no subcommand) auto-routes:
/// terminal reader in a TTY, one-shot ANSI when piped, PDF when `-o`/`--check`/
/// `--dump-typst`/`--open` is present.
#[derive(Debug, clap::Subcommand)]
pub enum Command {
    /// Render the Markdown file to a PDF.
    Pdf {
        /// Path to the Markdown file to render.
        input: Option<PathBuf>,
    },

    /// Read a Markdown file in the terminal with full styling.
    ///
    /// Launches a scrollable TUI in an interactive terminal and emits styled
    /// ANSI when piped or when `--plain` is set.
    #[cfg(feature = "terminal")]
    Read {
        /// Path to the Markdown file to read.
        input: Option<PathBuf>,
    },
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
        if self.dump_html && self.open {
            return Err(crate::error::SilkprintError::ConflictingOptions {
                details: "--open requires PDF output".to_string(),
            });
        }
        if self.dump_html && self.dump_typst {
            return Err(crate::error::SilkprintError::ConflictingOptions {
                details: "cannot combine --dump-html and --dump-typst".to_string(),
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

    /// The input file for the active mode: a subcommand's argument when present,
    /// otherwise the top-level positional.
    pub fn effective_input(&self) -> Option<PathBuf> {
        let from_command = match &self.command {
            Some(Command::Pdf { input }) => input.clone(),
            #[cfg(feature = "terminal")]
            Some(Command::Read { input }) => input.clone(),
            None => None,
        };
        from_command.or_else(|| self.input.clone())
    }

    /// Whether a PDF-output signal is present. In the bare form (no subcommand)
    /// this forces PDF rendering instead of the terminal reader.
    pub fn pdf_signaled(&self) -> bool {
        self.output.is_some() || self.check || self.dump_typst || self.dump_html || self.open
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
