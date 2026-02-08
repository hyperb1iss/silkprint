use clap::Parser;

use silkprint::cli::Cli;

fn main() -> miette::Result<()> {
    let cli = Cli::parse();
    cli.validate()?;

    // Stub — Wave 2A builds the full dispatch
    if cli.list_themes {
        let themes = silkprint::theme::builtin::list_themes();
        for t in &themes {
            #[allow(clippy::print_stdout)]
            {
                println!("  {} ({}) — {}", t.name, t.variant, t.description);
            }
        }
        return Ok(());
    }

    if cli.input.is_none() {
        return Err(miette::miette!("No input file specified. Run `silkprint --help` for usage."));
    }

    #[allow(clippy::print_stdout)]
    {
        println!("silkprint: not yet fully implemented — stubs in place");
    }

    Ok(())
}
