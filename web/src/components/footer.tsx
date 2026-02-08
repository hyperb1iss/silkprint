export function Footer() {
  return (
    <footer className="border-t border-sc-cyan/10 bg-sc-bg-dark">
      <div className="mx-auto flex max-w-7xl flex-col items-center gap-6 px-6 py-12 md:flex-row md:justify-between">
        <div className="flex flex-col items-center gap-1 md:items-start">
          <span className="gradient-text text-lg font-bold">SilkPrint</span>
          <span className="text-xs text-sc-fg-dim">Markdown to PDF, made stunning.</span>
        </div>

        <nav className="flex items-center gap-6 text-sm text-sc-fg-muted">
          <a
            href="https://github.com/hyperb1iss/silkprint"
            target="_blank"
            rel="noopener noreferrer"
            className="transition-colors hover:text-sc-cyan"
          >
            GitHub
          </a>
          <a
            href="https://github.com/hyperb1iss/silkprint#installation"
            target="_blank"
            rel="noopener noreferrer"
            className="transition-colors hover:text-sc-cyan"
          >
            CLI Docs
          </a>
          <a
            href="https://github.com/hyperb1iss/silkprint/issues"
            target="_blank"
            rel="noopener noreferrer"
            className="transition-colors hover:text-sc-cyan"
          >
            Issues
          </a>
        </nav>

        <div className="text-xs text-sc-fg-dim">
          &copy; {new Date().getFullYear()}{' '}
          <a
            href="https://github.com/hyperb1iss"
            target="_blank"
            rel="noopener noreferrer"
            className="text-sc-fg-muted transition-colors hover:text-sc-cyan"
          >
            hyperb1iss
          </a>
          . MIT License.
        </div>
      </div>
    </footer>
  );
}
