#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use clap::Parser;
use tracing::debug;

mod cli_ui;
mod theme_listing;

use silkprint::cli::Cli;
use silkprint::render::input::read_document_input;
use silkprint::warnings::SilkprintWarning;
use silkprint::{PaperSize, RenderOptions, ThemeSource};

use crate::cli_ui::{
    SEPARATOR, coral, cyan, dim, display_warnings, estimate_page_count, green, make_spinner,
    purple, setup_color, setup_miette, setup_tracing,
};

/// Handle `--check`: parse + validate only, no render.
fn handle_check(
    cli: &Cli,
    input_path: &std::path::Path,
    options: &RenderOptions,
) -> miette::Result<()> {
    let start = Instant::now();

    let input = read_document_input(input_path)?.body;

    // Run the full render pipeline so asset resolution and Typst compilation
    // are validated as well.
    let (_pdf_bytes, mut warnings) = silkprint::render(&input, Some(input_path), options)?;
    if cli.validate_links {
        append_link_warnings(&input, Some(input_path), &mut warnings);
    }
    let elapsed = start.elapsed();

    display_warnings(&warnings);

    eprintln!(
        "  {} {} validated in {:.0?}",
        green("\u{2713}"),
        input_path.display(),
        elapsed,
    );

    Ok(())
}

/// Handle `--dump-typst`: emit Typst markup to stdout or file.
fn handle_dump_typst(
    input_path: &std::path::Path,
    output: Option<&str>,
    options: &RenderOptions,
    quiet: bool,
) -> miette::Result<()> {
    let input = read_document_input(input_path)?.body;

    let (typst_source, warnings) =
        silkprint::render_to_typst_with_path(&input, Some(input_path), options)?;

    if !quiet {
        display_warnings(&warnings);
    }

    match output {
        Some(path) if path != "-" => {
            std::fs::write(path, &typst_source).map_err(|e| {
                silkprint::error::SilkprintError::OutputWrite {
                    path: path.to_string(),
                    source: e,
                }
            })?;
            if !quiet {
                eprintln!(
                    "  {} Typst source written to {}",
                    green("\u{2713}"),
                    cyan(path)
                );
            }
        }
        _ => {
            // Write to stdout
            io::stdout()
                .write_all(typst_source.as_bytes())
                .map_err(|e| silkprint::error::SilkprintError::OutputWrite {
                    path: "<stdout>".to_string(),
                    source: e,
                })?;
        }
    }

    Ok(())
}

fn handle_dump_html(cli: &Cli, input_path: &std::path::Path) -> miette::Result<()> {
    let input = read_document_input(input_path)?.body;
    let (html, warnings) =
        silkprint::render_to_html_with_path(&input, Some(input_path), cli.validate_links)?;

    if !cli.quiet {
        display_warnings(&warnings);
    }

    match cli.output.as_deref() {
        Some(path) if path != "-" => {
            std::fs::write(path, &html).map_err(|e| {
                silkprint::error::SilkprintError::OutputWrite {
                    path: path.to_string(),
                    source: e,
                }
            })?;
            if !cli.quiet {
                eprintln!("  {} HTML written to {}", green("\u{2713}"), cyan(path));
            }
        }
        _ => {
            io::stdout().write_all(html.as_bytes()).map_err(|e| {
                silkprint::error::SilkprintError::OutputWrite {
                    path: "<stdout>".to_string(),
                    source: e,
                }
            })?;
        }
    }

    Ok(())
}

/// Handle normal render mode: Markdown -> PDF.
#[allow(clippy::too_many_lines)]
fn handle_render(cli: &Cli, input_path: &Path, options: &RenderOptions) -> miette::Result<()> {
    let start = Instant::now();
    let verbose = cli.verbose > 0;
    let use_spinner = !cli.quiet && !verbose && io::stderr().is_terminal();

    if verbose {
        let version = env!("CARGO_PKG_VERSION");
        let sep = dim(SEPARATOR);
        eprintln!(
            "  {} {}",
            purple("\u{1f48e}"),
            purple(&format!("silkprint v{version}"))
        );
        eprintln!("  {sep}");
        eprintln!("  {} Parsing markdown", cyan("\u{26a1}"));
    }

    let spinner = if use_spinner {
        Some(make_spinner(&format!(
            "Rendering {} with {}",
            input_path
                .file_name()
                .map_or("input", |n| n.to_str().unwrap_or("input")),
            &cli.theme,
        )))
    } else {
        None
    };

    debug!("reading input: {}", input_path.display());
    let input = match read_document_input(input_path) {
        Ok(document) => document.body,
        Err(err) => {
            if let Some(ref sp) = spinner {
                sp.finish_and_clear();
            }
            return Err(err.into());
        }
    };

    if verbose {
        eprintln!(
            "  {} Applying theme       {}",
            cyan("\u{1f3a8}"),
            coral(&cli.theme)
        );
    }

    debug!("rendering with theme: {}", cli.theme);
    let render_result = silkprint::render(&input, Some(input_path), options);

    // Clear spinner before any output
    if let Some(ref sp) = spinner {
        sp.finish_and_clear();
    }

    let (pdf_bytes, mut warnings) = render_result?;
    if cli.validate_links {
        append_link_warnings(&input, Some(input_path), &mut warnings);
    }
    let output_path = cli.resolve_output_path(input_path);
    let page_count = estimate_page_count(&pdf_bytes);

    if verbose {
        let bar = "\u{2588}".repeat(page_count.min(20));
        eprintln!(
            "  {} Rendering pages    {} {}",
            cyan("\u{1f52e}"),
            green(&bar),
            coral(&page_count.to_string()),
        );
    }

    // Write output
    if let Some(path) = &output_path {
        if verbose {
            eprintln!(
                "  {} Writing PDF         {}",
                purple("\u{1f49c}"),
                cyan(&path.display().to_string()),
            );
        }
        debug!("writing PDF to: {}", path.display());
        std::fs::write(path, &pdf_bytes).map_err(|e| {
            silkprint::error::SilkprintError::OutputWrite {
                path: path.display().to_string(),
                source: e,
            }
        })?;
    } else {
        // stdout mode
        debug!("writing PDF to stdout");
        io::stdout().write_all(&pdf_bytes).map_err(|e| {
            silkprint::error::SilkprintError::OutputWrite {
                path: "<stdout>".to_string(),
                source: e,
            }
        })?;
    }

    let elapsed = start.elapsed();

    if !cli.quiet {
        display_warnings(&warnings);
    }

    // Summary output
    if verbose {
        let sep = dim(SEPARATOR);
        eprintln!("  {sep}");
        eprintln!(
            "  {} {} pages rendered in {:.0?}",
            green("\u{2713}"),
            page_count,
            elapsed,
        );
    } else if !cli.quiet {
        let display_path = output_path
            .as_ref()
            .map_or("<stdout>".to_string(), |p| p.display().to_string());
        eprintln!(
            "  {} {} ({} pages, {:.0?})",
            green("\u{2713}"),
            cyan(&display_path),
            page_count,
            elapsed,
        );
    }

    // Open in system viewer if requested
    if cli.open
        && let Some(ref path) = output_path
    {
        debug!("opening PDF: {}", path.display());
        open::that(path).map_err(|e| silkprint::error::SilkprintError::RenderFailed {
            details: format!("failed to open PDF viewer: {e}"),
            hint: "Check that a PDF viewer is installed and associated with .pdf files".to_string(),
        })?;
    }

    Ok(())
}

// ── Helpers ────────────────────────────────────────────────────

fn append_link_warnings(
    input: &str,
    input_path: Option<&std::path::Path>,
    warnings: &mut Vec<SilkprintWarning>,
) {
    let arena = comrak::Arena::new();
    let root = silkprint::render::markdown::parse(&arena, input);
    let mut collector = silkprint::warnings::WarningCollector::new();
    silkprint::render::linkcheck::validate_links(root, input_path, &mut collector);
    warnings.extend(collector.into_warnings());
}

/// Resolve the `ThemeSource` from the CLI `--theme` argument.
fn resolve_theme_source(theme_arg: &str) -> ThemeSource {
    let path = std::path::Path::new(theme_arg);
    if path.extension().is_some_and(|ext| ext == "toml") {
        ThemeSource::Custom(path.to_path_buf())
    } else {
        ThemeSource::BuiltIn(theme_arg.to_string())
    }
}

fn theme_flag_explicit() -> bool {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .any(|a| a == "--theme" || a.starts_with("--theme="))
        || args.windows(2).any(|w| w[0] == "-t")
        || args.iter().any(|a| a.starts_with("-t") && a.len() > 2)
}

/// Build `RenderOptions` from the parsed CLI arguments.
fn build_render_options(cli: &Cli) -> miette::Result<RenderOptions> {
    let paper = PaperSize::from_str_case_insensitive(&cli.paper)?;
    let theme = resolve_theme_source(&cli.theme);
    let font_dirs = cli.font_dir.iter().cloned().collect();

    Ok(RenderOptions {
        theme,
        theme_explicit: theme_flag_explicit(),
        paper,
        font_dirs,
        toc: cli.toc_override(),
        title_page: cli.title_page_override(),
    })
}

#[cfg(feature = "terminal")]
fn long_flag_explicit(name: &str) -> bool {
    let assignment = format!("{name}=");
    std::env::args().any(|arg| arg == name || arg.starts_with(&assignment))
}

#[cfg(feature = "terminal")]
fn env_string(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(feature = "terminal")]
fn env_u16(name: &str) -> Option<u16> {
    env_string(name).and_then(|value| value.parse().ok())
}

#[cfg(feature = "terminal")]
fn effective_reader_color(
    cli: &Cli,
    settings: &silkprint::render::terminal::config::ReaderSettings,
) -> String {
    if long_flag_explicit("--color") {
        return cli.color.clone();
    }
    env_string("SILKPRINT_COLOR")
        .or_else(|| settings.color().map(str::to_string))
        .unwrap_or_else(|| cli.color.clone())
}

#[cfg(feature = "terminal")]
fn effective_reader_width(
    cli: &Cli,
    settings: &silkprint::render::terminal::config::ReaderSettings,
) -> Option<u16> {
    cli.width
        .or_else(|| env_u16("SILKPRINT_WIDTH"))
        .or_else(|| settings.width())
}

#[cfg(feature = "terminal")]
fn effective_reader_glyphs(
    cli: &Cli,
    settings: &silkprint::render::terminal::config::ReaderSettings,
) -> Option<silkprint::GlyphTier> {
    cli.glyphs
        .as_deref()
        .and_then(silkprint::GlyphTier::parse)
        .or_else(|| {
            env_string("SILKPRINT_GLYPHS").and_then(|value| silkprint::GlyphTier::parse(&value))
        })
        .or_else(|| settings.glyphs().and_then(silkprint::GlyphTier::parse))
}

#[cfg(feature = "terminal")]
fn effective_reader_pager(
    settings: &silkprint::render::terminal::config::ReaderSettings,
) -> String {
    env_string("SILKPRINT_PAGER")
        .or_else(|| env_string("PAGER"))
        .or_else(|| settings.pager().map(str::to_string))
        .unwrap_or_else(|| "less -R".to_string())
}

/// Handle `read` mode: render Markdown to styled terminal output.
///
/// In an interactive terminal it launches the scrollable TUI; when piped, or
/// with `--plain`, it emits one-shot styled ANSI.
#[cfg(feature = "terminal")]
fn handle_read(cli: &Cli, input_path: &std::path::Path) -> miette::Result<()> {
    let document = read_document_input(input_path)?;
    let base_dir = silkprint::render::origin::local_base_dir(input_path);
    handle_read_source(
        cli,
        ReadSource {
            input: document.body,
            base_dir,
            watch_path: document.watch_path.clone(),
            origin: document
                .watch_path
                .map(silkprint::render::origin::DocumentOrigin::local),
        },
    )
}

#[cfg(feature = "terminal")]
fn handle_read_remote(
    cli: &Cli,
    raw: &str,
    input: &silkprint::render::remote::RemoteInput,
) -> miette::Result<()> {
    let remote = silkprint::render::remote::fetch_remote_document(input).map_err(|message| {
        silkprint::error::SilkprintError::RemoteFetch {
            url: raw.to_string(),
            message,
        }
    })?;
    handle_read_source(
        cli,
        ReadSource {
            input: remote.body,
            base_dir: None,
            watch_path: None,
            origin: Some(remote.origin),
        },
    )
}

#[cfg(feature = "terminal")]
struct ReadSource {
    input: String,
    base_dir: Option<PathBuf>,
    watch_path: Option<PathBuf>,
    origin: Option<silkprint::render::origin::DocumentOrigin>,
}

#[cfg(feature = "terminal")]
fn handle_read_source(cli: &Cli, source: ReadSource) -> miette::Result<()> {
    // Reading is its own mode; PDF-only flags don't apply.
    if cli.check || cli.open || cli.dump_typst || cli.dump_html || cli.output.is_some() {
        return Err(silkprint::error::SilkprintError::ConflictingOptions {
            details:
                "--check, --open, --dump-typst, --dump-html, and --output do not apply when reading"
                    .to_string(),
        }
        .into());
    }
    // Reject an unrecognized --glyphs value instead of silently falling back.
    if let Some(value) = cli.glyphs.as_deref()
        && silkprint::GlyphTier::parse(value).is_none()
    {
        return Err(silkprint::error::SilkprintError::ConflictingOptions {
            details: format!("unknown --glyphs '{value}' (expected nerdfont, unicode, or ascii)"),
        }
        .into());
    }

    let reader_settings = silkprint::render::terminal::config::load_settings();
    let glyph_tier = effective_reader_glyphs(cli, &reader_settings);
    let mut options = build_render_options(cli)?;
    if !options.theme_explicit {
        if let Some(theme) = env_string("SILKPRINT_THEME") {
            options.theme = resolve_theme_source(&theme);
            options.theme_explicit = true;
        } else if let Some(theme) = reader_settings.user_theme() {
            options.theme = resolve_theme_source(theme);
        } else if io::stdout().is_terminal()
            && let Some(tone) = silkprint::render::terminal::caps::detect_background_tone()
        {
            options.theme = silkprint::ThemeSource::BuiltIn(tone.silk_default_theme().to_string());
        } else if let Some(theme) = reader_settings.reader_theme() {
            options.theme = resolve_theme_source(theme);
        }
    }

    // Interactive TTY → TUI; piped or --plain → one-shot. Both resolve the
    // effective theme the same way (front matter / path / builtin).
    if io::stdout().is_terminal() && !cli.plain {
        let (theme, theme_name, _warnings) =
            silkprint::resolve_terminal_theme(&source.input, &options)?;
        silkprint::run_terminal_tui(
            &source.input,
            theme,
            &theme_name,
            silkprint::TerminalTuiOptions {
                glyph_override: glyph_tier,
                images: !cli.no_images,
                base_dir: source.base_dir,
                origin: source.origin,
                watch_path: source.watch_path,
                font_dirs: options.font_dirs.clone(),
                settings: Some(reader_settings.clone()),
            },
        )
        .map_err(|e| silkprint::error::SilkprintError::RenderFailed {
            details: e.to_string(),
            hint: "the terminal reader could not start".to_string(),
        })?;
        return Ok(());
    }

    let terminal_options = silkprint::TerminalRenderOptions {
        color: silkprint::ColorChoice::parse(&effective_reader_color(cli, &reader_settings)),
        glyphs: glyph_tier,
        images: !cli.no_images,
        width: effective_reader_width(cli, &reader_settings),
    };

    let (output, warnings) = silkprint::render_to_terminal_with_origin(
        &source.input,
        source.origin.as_ref(),
        &options,
        &terminal_options,
    )?;

    emit_one_shot_output(cli, &reader_settings, &output);

    if !cli.quiet {
        display_warnings(&warnings);
    }
    Ok(())
}

#[cfg(feature = "terminal")]
fn emit_one_shot_output(
    cli: &Cli,
    settings: &silkprint::render::terminal::config::ReaderSettings,
    output: &str,
) {
    if should_page_output(
        output,
        io::stdout().is_terminal(),
        cli.no_pager,
        terminal_height(),
    ) && page_output(output, &effective_reader_pager(settings)).is_ok()
    {
        return;
    }

    print!("{output}");
    io::stdout().flush().ok();
}

#[cfg(feature = "terminal")]
fn should_page_output(
    output: &str,
    stdout_is_tty: bool,
    no_pager: bool,
    terminal_height: Option<u16>,
) -> bool {
    if !stdout_is_tty || no_pager {
        return false;
    }
    let Some(height) = terminal_height else {
        return false;
    };
    output.lines().count() > usize::from(height.max(1))
}

#[cfg(feature = "terminal")]
fn terminal_height() -> Option<u16> {
    ratatui::crossterm::terminal::size()
        .ok()
        .map(|(_width, height)| height)
}

#[cfg(feature = "terminal")]
fn page_output(output: &str, pager: &str) -> io::Result<()> {
    use std::process::{Command, Stdio};

    let mut parts = pager.split_whitespace();
    let Some(program) = parts.next() else {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "empty pager"));
    };
    let mut child = Command::new(program)
        .args(parts)
        .stdin(Stdio::piped())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(output.as_bytes());
    }
    let _ = child.wait();
    Ok(())
}

#[cfg(feature = "terminal")]
fn parse_remote_read_input(
    input: Option<&PathBuf>,
) -> miette::Result<Option<(String, silkprint::render::remote::RemoteInput)>> {
    let Some(input) = input else {
        return Ok(None);
    };
    let raw = input.to_string_lossy().into_owned();
    silkprint::render::remote::parse_remote_input(&raw)
        .map(|remote| remote.map(|remote| (raw.clone(), remote)))
        .map_err(|message| {
            silkprint::error::SilkprintError::RemoteFetch { url: raw, message }.into()
        })
}

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

/// Render the input to a PDF, dispatching the `--check` / `--dump-typst`
/// sub-modes that share the PDF pipeline.
fn run_pdf(cli: &Cli, input_path: &Path) -> miette::Result<()> {
    let options = build_render_options(cli)?;
    if cli.check {
        return handle_check(cli, input_path, &options);
    }
    if cli.dump_typst {
        return handle_dump_typst(input_path, cli.output.as_deref(), &options, cli.quiet);
    }
    if cli.dump_html {
        return handle_dump_html(cli, input_path);
    }
    handle_render(cli, input_path, &options)
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
            if let Some((raw, remote)) = parse_remote_read_input(effective_input.as_ref())? {
                return handle_read_remote(&cli, &raw, &remote);
            }
            let input = require_input(effective_input)?;
            return handle_read(&cli, &input);
        }
        None => {}
    }

    // Bare form: read in the terminal by default; a PDF flag (-o / --check /
    // --dump-typst / --open) routes to PDF rendering instead.
    #[cfg(feature = "terminal")]
    if !cli.pdf_signaled() {
        if let Some((raw, remote)) = parse_remote_read_input(cli.input.as_ref())? {
            return handle_read_remote(&cli, &raw, &remote);
        }
        let input = require_input(cli.input.clone())?;
        return handle_read(&cli, &input);
    }
    let input = require_input(cli.input.clone())?;
    run_pdf(&cli, &input)
}

#[cfg(all(test, feature = "terminal"))]
mod tests {
    use super::should_page_output;

    #[test]
    fn pages_only_tty_output_that_exceeds_height() {
        let output = "one\ntwo\nthree\n";

        assert!(should_page_output(output, true, false, Some(2)));
        assert!(!should_page_output(output, true, false, Some(3)));
        assert!(!should_page_output(output, false, false, Some(2)));
        assert!(!should_page_output(output, true, true, Some(2)));
        assert!(!should_page_output(output, true, false, None));
    }
}
