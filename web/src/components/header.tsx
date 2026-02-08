import Link from 'next/link';

export function Header() {
  return (
    <header className="glass fixed top-0 z-50 w-full">
      <div className="mx-auto flex max-w-7xl items-center justify-between px-6 py-4">
        <Link href="/" className="flex items-center gap-2">
          <span className="gradient-text text-xl font-bold tracking-tight">SilkPrint</span>
        </Link>

        <nav className="flex items-center gap-6">
          <a
            href="#editor"
            className="text-sm text-sc-fg-muted transition-colors hover:text-sc-cyan"
          >
            Editor
          </a>
          <a
            href="#themes"
            className="text-sm text-sc-fg-muted transition-colors hover:text-sc-cyan"
          >
            Themes
          </a>
          <a
            href="https://github.com/hyperb1iss/silkprint"
            target="_blank"
            rel="noopener noreferrer"
            className="text-sm text-sc-fg-muted transition-colors hover:text-sc-cyan"
          >
            GitHub
          </a>
          <a
            href="https://github.com/hyperb1iss/silkprint#installation"
            target="_blank"
            rel="noopener noreferrer"
            className="rounded-lg bg-sc-bg-highlight px-4 py-2 text-sm font-medium text-sc-cyan transition-all hover:-translate-y-0.5 hover:shadow-[0_0_20px_rgba(128,255,234,0.15)]"
          >
            Install CLI
          </a>
        </nav>
      </div>
    </header>
  );
}
