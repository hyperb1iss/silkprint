'use client';

import Link from 'next/link';
import { useCallback, useEffect, useState } from 'react';

export function Header() {
  const [menuOpen, setMenuOpen] = useState(false);

  // Close menu on resize past mobile breakpoint
  useEffect(() => {
    const mq = window.matchMedia('(min-width: 768px)');
    const handler = () => {
      if (mq.matches) setMenuOpen(false);
    };
    mq.addEventListener('change', handler);
    return () => mq.removeEventListener('change', handler);
  }, []);

  // Lock body scroll when menu is open
  useEffect(() => {
    document.body.style.overflow = menuOpen ? 'hidden' : '';
    return () => {
      document.body.style.overflow = '';
    };
  }, [menuOpen]);

  const closeMenu = useCallback(() => setMenuOpen(false), []);

  return (
    <header className="glass fixed top-0 z-50 w-full">
      <div className="mx-auto flex max-w-7xl items-center justify-between px-4 py-3 md:px-6 md:py-4">
        <Link href="/" className="flex items-center gap-2">
          <span className="gradient-text text-xl font-bold tracking-tight">SilkPrint</span>
        </Link>

        {/* Desktop nav */}
        <nav className="hidden items-center gap-6 md:flex">
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

        {/* Mobile hamburger */}
        <button
          type="button"
          onClick={() => setMenuOpen(v => !v)}
          className="flex items-center justify-center rounded-lg p-2 text-sc-fg-muted transition-colors hover:bg-sc-bg-highlight hover:text-sc-fg md:hidden"
          aria-label={menuOpen ? 'Close menu' : 'Open menu'}
        >
          <svg
            aria-hidden="true"
            className="h-5 w-5"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            strokeWidth={2}
          >
            {menuOpen ? (
              <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
            ) : (
              <path strokeLinecap="round" strokeLinejoin="round" d="M4 6h16M4 12h16M4 18h16" />
            )}
          </svg>
        </button>
      </div>

      {/* Mobile drawer */}
      {menuOpen && (
        <nav className="animate-drop-in border-t border-sc-cyan/10 bg-sc-bg-dark/95 backdrop-blur-xl md:hidden">
          <div className="flex flex-col gap-1 px-4 py-4">
            {/* biome-ignore lint/a11y/useValidAnchor: hash nav + menu close */}
            <a
              href="#editor"
              onClick={closeMenu}
              className="rounded-lg px-3 py-2.5 text-sm font-medium text-sc-fg-muted transition-colors hover:bg-sc-bg-highlight hover:text-sc-cyan"
            >
              Editor
            </a>
            {/* biome-ignore lint/a11y/useValidAnchor: hash nav + menu close */}
            <a
              href="#themes"
              onClick={closeMenu}
              className="rounded-lg px-3 py-2.5 text-sm font-medium text-sc-fg-muted transition-colors hover:bg-sc-bg-highlight hover:text-sc-cyan"
            >
              Themes
            </a>
            <a
              href="https://github.com/hyperb1iss/silkprint"
              target="_blank"
              rel="noopener noreferrer"
              className="rounded-lg px-3 py-2.5 text-sm font-medium text-sc-fg-muted transition-colors hover:bg-sc-bg-highlight hover:text-sc-cyan"
            >
              GitHub
            </a>
            <div className="mt-2 border-t border-sc-cyan/10 pt-3">
              <a
                href="https://github.com/hyperb1iss/silkprint#installation"
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center justify-center rounded-lg bg-sc-bg-highlight px-4 py-2.5 text-sm font-medium text-sc-cyan transition-all hover:shadow-[0_0_20px_rgba(128,255,234,0.15)]"
              >
                Install CLI
              </a>
            </div>
          </div>
        </nav>
      )}
    </header>
  );
}
