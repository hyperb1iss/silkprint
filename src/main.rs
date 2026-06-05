#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::io;
use std::path::PathBuf;

use clap::Parser;

mod cli_ui;
mod pdf_mode;
#[cfg(feature = "terminal")]
mod reader_mode;
mod theme_listing;

use silkprint::cli::Cli;

use crate::cli_ui::{setup_color, setup_miette, setup_tracing};
use crate::pdf_mode::run_pdf;

/// Resolve the input file for a mode, erroring if absent or missing on disk.
fn require_input(input: Option<PathBuf>) -> miette::Result<PathBuf> {
    let path = input.ok_or_else(|| {
        miette::miette!("No input file specified. Run `silkprint --help` for usage.")
    })?;
    if !path.exists() {
        return Err(silkprint::error::SilkprintError::InputRead {
            path: path.display().to_string(),
            source: io::Error::new(io::ErrorKind::NotFound, "file not found"),
        }
        .into());
    }
    Ok(path)
}

// ── Entrypoint ─────────────────────────────────────────────────

fn main() -> miette::Result<()> {
    let cli = Cli::parse();

    // Configure subsystems
    setup_color(&cli.color);
    setup_miette();
    setup_tracing(cli.verbose, cli.quiet);

    // Validate flag conflicts
    cli.validate()?;

    // ── Mode dispatch ──────────────────────────────────────────

    // --list-themes: standalone mode, no input required
    if cli.list_themes {
        theme_listing::handle_list_themes();
        return Ok(());
    }

    // Explicit subcommand pins the mode; input may live on the subcommand.
    match &cli.command {
        Some(silkprint::cli::Command::Pdf { .. }) => {
            let input = require_input(cli.effective_input())?;
            return run_pdf(&cli, &input);
        }
        #[cfg(feature = "terminal")]
        Some(silkprint::cli::Command::Read { .. }) => {
            let effective_input = cli.effective_input();
            if let Some((raw, remote)) =
                reader_mode::parse_remote_read_input(effective_input.as_ref())?
            {
                return reader_mode::handle_read_remote(&cli, &raw, &remote);
            }
            let input = require_input(effective_input)?;
            return reader_mode::handle_read(&cli, &input);
        }
        None => {}
    }

    // Bare form: read in the terminal by default; a PDF flag (-o / --check /
    // --dump-typst / --open) routes to PDF rendering instead.
    #[cfg(feature = "terminal")]
    if !cli.pdf_signaled() {
        if let Some((raw, remote)) = reader_mode::parse_remote_read_input(cli.input.as_ref())? {
            return reader_mode::handle_read_remote(&cli, &raw, &remote);
        }
        let input = require_input(cli.input.clone())?;
        return reader_mode::handle_read(&cli, &input);
    }
    let input = require_input(cli.input.clone())?;
    run_pdf(&cli, &input)
}
