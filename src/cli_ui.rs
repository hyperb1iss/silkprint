use std::io::{self, IsTerminal};
use std::sync::atomic::{AtomicBool, Ordering};

use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;

use silkprint::warnings::SilkprintWarning;

static USE_COLOR: AtomicBool = AtomicBool::new(true);

pub(crate) fn color_enabled() -> bool {
    USE_COLOR.load(Ordering::Relaxed)
}

pub(crate) fn purple(s: &str) -> String {
    if color_enabled() {
        format!("{}", s.truecolor(225, 53, 255).bold())
    } else {
        s.to_string()
    }
}

pub(crate) fn cyan(s: &str) -> String {
    if color_enabled() {
        format!("{}", s.truecolor(128, 255, 234))
    } else {
        s.to_string()
    }
}

pub(crate) fn coral(s: &str) -> String {
    if color_enabled() {
        format!("{}", s.truecolor(255, 106, 193))
    } else {
        s.to_string()
    }
}

pub(crate) fn yellow(s: &str) -> String {
    if color_enabled() {
        format!("{}", s.truecolor(241, 250, 140))
    } else {
        s.to_string()
    }
}

pub(crate) fn green(s: &str) -> String {
    if color_enabled() {
        format!("{}", s.truecolor(80, 250, 123))
    } else {
        s.to_string()
    }
}

pub(crate) fn dim(s: &str) -> String {
    if color_enabled() {
        format!("{}", s.dimmed())
    } else {
        s.to_string()
    }
}

pub(crate) fn setup_color(mode: &str) {
    let enabled = match mode {
        "always" => true,
        "never" => false,
        _ => io::stderr().is_terminal(),
    };
    USE_COLOR.store(enabled, Ordering::Relaxed);
}

pub(crate) fn setup_miette() {
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
    .ok();
}

pub(crate) fn setup_tracing(verbose: u8, quiet: bool) {
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

pub(crate) const SEPARATOR: &str = "\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}";

pub(crate) fn make_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    let style = if color_enabled() {
        ProgressStyle::default_spinner()
            .tick_strings(&[
                "\u{2801}", "\u{2809}", "\u{2819}", "\u{281b}", "\u{2813}", "\u{2816}", "\u{2826}",
                "\u{2834}", "\u{2830}", "\u{2820}", "\u{2800}", "\u{2801}",
            ])
            .template("  \x1b[38;2;225;53;255m{spinner}\x1b[0m {msg}")
    } else {
        ProgressStyle::default_spinner()
            .tick_strings(&["|", "/", "-", "\\", "|"])
            .template("  {spinner} {msg}")
    };
    if let Ok(s) = style {
        pb.set_style(s);
    }
    pb.set_message(message.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(80));
    pb
}

pub(crate) fn display_warnings(warnings: &[SilkprintWarning]) {
    for w in warnings {
        eprintln!("  {} {}", yellow("\u{26a0}"), strip_control(&w.to_string()));
    }
}

fn strip_control(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control() || *c == '\t')
        .collect()
}

pub(crate) fn estimate_page_count(pdf_bytes: &[u8]) -> usize {
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
    count.max(1)
}
