use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

use silkprint::cli::Cli;
use silkprint::render::input::read_document_input;

use crate::cli_ui::display_warnings;
use crate::pdf_mode::{build_render_options, resolve_theme_source};

fn long_flag_explicit(name: &str) -> bool {
    let assignment = format!("{name}=");
    std::env::args().any(|arg| arg == name || arg.starts_with(&assignment))
}

fn env_string(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn env_u16(name: &str) -> Option<u16> {
    env_string(name).and_then(|value| value.parse().ok())
}

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

fn effective_reader_width(
    cli: &Cli,
    settings: &silkprint::render::terminal::config::ReaderSettings,
) -> Option<u16> {
    cli.width
        .or_else(|| env_u16("SILKPRINT_WIDTH"))
        .or_else(|| settings.width())
}

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

fn effective_reader_pager(
    settings: &silkprint::render::terminal::config::ReaderSettings,
) -> String {
    env_string("SILKPRINT_PAGER")
        .or_else(|| env_string("PAGER"))
        .or_else(|| settings.pager().map(str::to_string))
        .unwrap_or_else(|| "less -R".to_string())
}

pub(crate) fn handle_read(cli: &Cli, input_path: &Path) -> miette::Result<()> {
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

pub(crate) fn handle_read_remote(
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

struct ReadSource {
    input: String,
    base_dir: Option<PathBuf>,
    watch_path: Option<PathBuf>,
    origin: Option<silkprint::render::origin::DocumentOrigin>,
}

fn handle_read_source(cli: &Cli, source: ReadSource) -> miette::Result<()> {
    if cli.check || cli.open || cli.dump_typst || cli.dump_html || cli.output.is_some() {
        return Err(silkprint::error::SilkprintError::ConflictingOptions {
            details:
                "--check, --open, --dump-typst, --dump-html, and --output do not apply when reading"
                    .to_string(),
        }
        .into());
    }
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

fn terminal_height() -> Option<u16> {
    ratatui::crossterm::terminal::size()
        .ok()
        .map(|(_width, height)| height)
}

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

pub(crate) fn parse_remote_read_input(
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

#[cfg(test)]
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
