#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use clap::Parser;
use owo_colors::OwoColorize;
use tracing::debug;

use silkprint::cli::Cli;
use silkprint::warnings::SilkprintWarning;
use silkprint::{PaperSize, RenderOptions, ThemeSource};

// ── Color control ──────────────────────────────────────────────

/// Global flag for whether colored output is enabled.
static USE_COLOR: AtomicBool = AtomicBool::new(true);

fn color_enabled() -> bool {
    USE_COLOR.load(Ordering::Relaxed)
}

/// Apply `SilkCircuit` Electric Purple (bold) to a string.
fn purple(s: &str) -> String {
    if color_enabled() {
        format!("{}", s.truecolor(225, 53, 255).bold())
    } else {
        s.to_string()
    }
}

/// Apply `SilkCircuit` Neon Cyan to a string.
fn cyan(s: &str) -> String {
    if color_enabled() {
        format!("{}", s.truecolor(128, 255, 234))
    } else {
        s.to_string()
    }
}

/// Apply `SilkCircuit` Coral to a string.
fn coral(s: &str) -> String {
    if color_enabled() {
        format!("{}", s.truecolor(255, 106, 193))
    } else {
        s.to_string()
    }
}

/// Apply `SilkCircuit` Electric Yellow to a string.
fn yellow(s: &str) -> String {
    if color_enabled() {
        format!("{}", s.truecolor(241, 250, 140))
    } else {
        s.to_string()
    }
}

/// Apply `SilkCircuit` Success Green to a string.
fn green(s: &str) -> String {
    if color_enabled() {
        format!("{}", s.truecolor(80, 250, 123))
    } else {
        s.to_string()
    }
}

/// Apply dim styling to a string.
fn dim(s: &str) -> String {
    if color_enabled() {
        format!("{}", s.dimmed())
    } else {
        s.to_string()
    }
}

// ── Setup ──────────────────────────────────────────────────────

/// Configure the color mode based on `--color` flag value.
fn setup_color(mode: &str) {
    let enabled = match mode {
        "always" => true,
        "never" => false,
        // "auto" -- color when stderr is a terminal
        _ => io::stderr().is_terminal(),
    };
    USE_COLOR.store(enabled, Ordering::Relaxed);
}

/// Install `miette` as the global error report handler with fancy output.
fn setup_miette() {
    miette::set_hook(Box::new(|_| {
        Box::new(
            miette::MietteHandlerOpts::new()
                .terminal_links(true)
                .unicode(true)
                .context_lines(2)
                .tab_width(4)
                .build(),
        )
    }))
    .ok(); // Ignore if already set (e.g. in tests)
}

/// Initialize tracing-subscriber based on verbosity level.
///
/// - quiet: no tracing
/// - v=0: warn
/// - v=1: info
/// - v=2: debug
/// - v=3+: trace
fn setup_tracing(verbose: u8, quiet: bool) {
    use tracing_subscriber::EnvFilter;

    if quiet {
        return;
    }

    let filter = match verbose {
        0 => "silkprint=warn",
        1 => "silkprint=info",
        2 => "silkprint=debug",
        _ => "silkprint=trace",
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(filter)),
        )
        .with_writer(io::stderr)
        .without_time()
        .init();
}

// ── Separator constant ─────────────────────────────────────────

const SEPARATOR: &str = "\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}";

// ── Mode handlers ──────────────────────────────────────────────

/// Handle `--list-themes`: display all built-in themes and exit.
fn handle_list_themes() {
    let themes = silkprint::theme::builtin::list_themes();
    let sep = dim(SEPARATOR);

    println!("  {} {}", purple("\u{1f48e}"), purple("Available Themes"));
    println!("  {sep}");

    for t in &themes {
        let badge = if t.print_safe {
            green("\u{25cf}")
        } else {
            yellow("\u{25cb}")
        };
        let name = cyan(t.name);
        let variant = dim(&format!("({})", t.variant));
        let desc = &t.description;
        println!("  {badge} {name:<28} {variant:<10} {desc}");
    }

    println!("  {sep}");
    let count = themes.len();
    println!(
        "  {} themes available  {} = print-safe",
        coral(&count.to_string()),
        green("\u{25cf}"),
    );
}

/// Handle `--check`: parse + validate only, no render.
fn handle_check(input_path: &std::path::Path, options: &RenderOptions) -> miette::Result<()> {
    let start = Instant::now();

    let input = std::fs::read_to_string(input_path).map_err(|e| {
        silkprint::error::SilkprintError::InputRead {
            path: input_path.display().to_string(),
            source: e,
        }
    })?;

    // Run the full pipeline up to Typst source generation (validates everything)
    let (_typst_source, warnings) = silkprint::render_to_typst(&input, options)?;
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
    let input = std::fs::read_to_string(input_path).map_err(|e| {
        silkprint::error::SilkprintError::InputRead {
            path: input_path.display().to_string(),
            source: e,
        }
    })?;

    let (typst_source, warnings) = silkprint::render_to_typst(&input, options)?;

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

/// Handle normal render mode: Markdown -> PDF.
#[allow(clippy::too_many_lines)]
fn handle_render(cli: &Cli, input_path: &PathBuf, options: &RenderOptions) -> miette::Result<()> {
    let start = Instant::now();
    let verbose = cli.verbose > 0;

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

    debug!("reading input: {}", input_path.display());
    let input = std::fs::read_to_string(input_path).map_err(|e| {
        silkprint::error::SilkprintError::InputRead {
            path: input_path.display().to_string(),
            source: e,
        }
    })?;

    if verbose {
        eprintln!(
            "  {} Applying theme       {}",
            cyan("\u{1f3a8}"),
            coral(&cli.theme)
        );
    }

    debug!("rendering with theme: {}", cli.theme);
    let (pdf_bytes, warnings) = silkprint::render(&input, Some(input_path.as_path()), options)?;

    let output_path = cli.resolve_output_path(input_path);

    // Calculate page count -- rough heuristic from PDF bytes
    // (the real page count comes from the Typst compilation, but for now
    // we estimate from the rendered output)
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
            "silkprint: {} ({} pages, {:.0?})",
            cyan(&display_path),
            page_count,
            elapsed,
        );
    }

    // Open in system viewer if requested
    if cli.open {
        if let Some(ref path) = output_path {
            debug!("opening PDF: {}", path.display());
            open::that(path).map_err(|e| silkprint::error::SilkprintError::RenderFailed {
                details: format!("failed to open PDF viewer: {e}"),
                hint: "Check that a PDF viewer is installed and associated with .pdf files"
                    .to_string(),
            })?;
        }
    }

    Ok(())
}

// ── Helpers ────────────────────────────────────────────────────

/// Display warnings to stderr with `SilkCircuit` styling.
fn display_warnings(warnings: &[SilkprintWarning]) {
    for w in warnings {
        eprintln!("  {} {}", yellow("\u{26a0}"), w);
    }
}

/// Estimate page count from PDF bytes by counting page object markers.
///
/// This is a quick heuristic -- the Typst compiler knows the real count,
/// but until we plumb that through, this gets us close.
fn estimate_page_count(pdf_bytes: &[u8]) -> usize {
    // Look for /Type /Page entries (not /Pages)
    let needle = b"/Type /Page";
    let anti = b"/Type /Pages";
    let mut count = 0;
    let mut pos = 0;
    while pos + anti.len() <= pdf_bytes.len() {
        if pdf_bytes[pos..].starts_with(needle) && !pdf_bytes[pos..].starts_with(anti) {
            count += 1;
            pos += needle.len();
        } else {
            pos += 1;
        }
    }
    count.max(1) // At least 1 page
}

/// Resolve the `ThemeSource` from the CLI `--theme` argument.
fn resolve_theme_source(theme_arg: &str) -> ThemeSource {
    let path = std::path::Path::new(theme_arg);
    if path.extension().is_some_and(|ext| ext == "toml") && path.exists() {
        ThemeSource::Custom(path.to_path_buf())
    } else {
        ThemeSource::BuiltIn(theme_arg.to_string())
    }
}

/// Build `RenderOptions` from the parsed CLI arguments.
fn build_render_options(cli: &Cli) -> miette::Result<RenderOptions> {
    let paper = PaperSize::from_str_case_insensitive(&cli.paper)?;
    let theme = resolve_theme_source(&cli.theme);
    let font_dirs = cli.font_dir.iter().cloned().collect();

    // Detect if --theme / -t was explicitly passed (vs. clap default).
    // clap's -t always requires a value, so valid forms are:
    //   --theme NAME, --theme=NAME, -t NAME, -tNAME
    let theme_explicit = {
        let args: Vec<String> = std::env::args().collect();
        args.iter().any(|a| a == "--theme" || a.starts_with("--theme="))
            || args.windows(2).any(|w| w[0] == "-t")
            || args.iter().any(|a| a.starts_with("-t") && a.len() > 2)
    };

    Ok(RenderOptions {
        theme,
        theme_explicit,
        paper,
        font_dirs,
        toc: cli.toc_override(),
        title_page: cli.title_page_override(),
    })
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
        handle_list_themes();
        return Ok(());
    }

    // All other modes require an input file
    let input_path = cli.input.clone().ok_or_else(|| {
        miette::miette!("No input file specified. Run `silkprint --help` for usage.")
    })?;

    if !input_path.exists() {
        return Err(silkprint::error::SilkprintError::InputRead {
            path: input_path.display().to_string(),
            source: io::Error::new(io::ErrorKind::NotFound, "file not found"),
        }
        .into());
    }

    let options = build_render_options(&cli)?;

    // --check: validate only
    if cli.check {
        return handle_check(&input_path, &options);
    }

    // --dump-typst: emit Typst source
    if cli.dump_typst {
        return handle_dump_typst(&input_path, cli.output.as_deref(), &options, cli.quiet);
    }

    // Normal render: Markdown -> PDF
    handle_render(&cli, &input_path, &options)
}
