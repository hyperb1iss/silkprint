use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::time::Instant;

use tracing::debug;

use silkprint::cli::Cli;
use silkprint::render::input::read_document_input;
use silkprint::warnings::SilkprintWarning;
use silkprint::{PaperSize, RenderOptions, ThemeSource};

use crate::cli_ui::{
    SEPARATOR, coral, cyan, dim, display_warnings, estimate_page_count, green, make_spinner, purple,
};

fn handle_check(cli: &Cli, input_path: &Path, options: &RenderOptions) -> miette::Result<()> {
    let start = Instant::now();

    let input = read_document_input(input_path)?.body;

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

fn handle_dump_typst(
    input_path: &Path,
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

fn handle_dump_html(cli: &Cli, input_path: &Path) -> miette::Result<()> {
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

fn append_link_warnings(
    input: &str,
    input_path: Option<&Path>,
    warnings: &mut Vec<SilkprintWarning>,
) {
    let arena = comrak::Arena::new();
    let root = silkprint::render::markdown::parse(&arena, input);
    let mut collector = silkprint::warnings::WarningCollector::new();
    silkprint::render::linkcheck::validate_links(root, input_path, &mut collector);
    warnings.extend(collector.into_warnings());
}

pub(crate) fn resolve_theme_source(theme_arg: &str) -> ThemeSource {
    let path = Path::new(theme_arg);
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

pub(crate) fn build_render_options(cli: &Cli) -> miette::Result<RenderOptions> {
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

pub(crate) fn run_pdf(cli: &Cli, input_path: &Path) -> miette::Result<()> {
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
