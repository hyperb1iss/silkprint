import type { ReactNode } from 'react';

const ICON = {
  terminal: (
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      d="M6.75 7.5l3 2.25-3 2.25m4.5 0h3m-9 8.25h13.5A2.25 2.25 0 0021 18V6a2.25 2.25 0 00-2.25-2.25H5.25A2.25 2.25 0 003 6v12a2.25 2.25 0 002.25 2.25z"
    />
  ),
  palette: (
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      d="M9.53 16.122a3 3 0 00-5.78 1.128 2.25 2.25 0 01-2.4 2.245 4.5 4.5 0 008.4-2.245c0-.399-.078-.78-.22-1.128zm0 0a15.998 15.998 0 003.388-1.62m-5.043-.025a15.994 15.994 0 011.622-3.395m3.42 3.42a15.995 15.995 0 004.764-4.648l3.876-5.814a1.151 1.151 0 00-1.597-1.597L14.146 6.32a15.996 15.996 0 00-4.649 4.763m3.42 3.42a6.776 6.776 0 00-3.42-3.42"
    />
  ),
  photo: (
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      d="M2.25 15.75l5.159-5.159a2.25 2.25 0 013.182 0l5.159 5.159m-1.5-1.5l1.409-1.409a2.25 2.25 0 013.182 0l2.909 2.909m-18 3.75h16.5a1.5 1.5 0 001.5-1.5V6a1.5 1.5 0 00-1.5-1.5H3.75A1.5 1.5 0 002.25 6v12a1.5 1.5 0 001.5 1.5zm10.5-11.25h.008v.008h-.008V8.25zm.375 0a.375.375 0 11-.75 0 .375.375 0 01.75 0z"
    />
  ),
  book: (
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      d="M12 6.042A8.967 8.967 0 006 3.75c-1.052 0-2.062.18-3 .512v14.25A8.987 8.987 0 016 18c2.305 0 4.408.867 6 2.292m0-14.25a8.966 8.966 0 016-2.292c1.052 0 2.062.18 3 .512v14.25A8.987 8.987 0 0018 18a8.967 8.967 0 00-6 2.292m0-14.25v14.25"
    />
  ),
  bolt: (
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      d="M3.75 13.5l10.5-11.25L12 10.5h8.25L9.75 21.75 12 13.5H3.75z"
    />
  ),
  branch: (
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      d="M17.25 6.75L22.5 12l-5.25 5.25m-10.5 0L1.5 12l5.25-5.25m7.5-3l-4.5 16.5"
    />
  ),
};

function Glyph({ children }: { children: ReactNode }) {
  return (
    <svg
      aria-hidden="true"
      className="h-6 w-6"
      fill="none"
      viewBox="0 0 24 24"
      stroke="currentColor"
      strokeWidth={1.5}
    >
      {children}
    </svg>
  );
}

const FEATURES = [
  {
    icon: ICON.terminal,
    title: 'One engine, two destinations',
    description:
      'Read Markdown in a rich terminal reader, or render a publication-ready PDF — same parser, same themes, same result.',
  },
  {
    icon: ICON.palette,
    title: '40 themes, one palette',
    description:
      'SilkCircuit neon to Nord, Dracula, and print-perfect Academic. Every theme styles the reader and the page identically.',
  },
  {
    icon: ICON.photo,
    title: 'Inline images & diagrams',
    description:
      'Render images via Kitty, iTerm2 & Sixel, plus mermaid diagrams — right in the terminal flow, with graceful fallbacks.',
  },
  {
    icon: ICON.book,
    title: 'Full Markdown support',
    description:
      'Tables, math, footnotes, task lists, GitHub-style alerts, description lists, wikilinks, and emoji shortcodes.',
  },
  {
    icon: ICON.bolt,
    title: 'Powered by Typst + Rust',
    description:
      'No LaTeX. PDFs compile in milliseconds, the reader is built on ratatui — and the whole engine runs in your browser via wasm.',
  },
  {
    icon: ICON.branch,
    title: 'Open source',
    description: 'MIT licensed, built in Rust for speed and safety. Contributions welcome.',
  },
];

export function Features() {
  return (
    <section className="mx-auto max-w-7xl px-4 py-12 sm:px-6 md:py-20">
      <div className="mb-8 text-center md:mb-12">
        <h2 className="mb-2 text-2xl font-bold tracking-tight sm:text-3xl md:mb-3 md:text-4xl">
          Why <span className="gradient-text">SilkPrint</span>?
        </h2>
        <p className="mx-auto max-w-lg text-sc-fg-muted">
          Everything you need to read and publish Markdown beautifully, nothing you don&apos;t.
        </p>
      </div>

      <div className="grid grid-cols-1 gap-4 sm:gap-6 md:grid-cols-2 lg:grid-cols-3">
        {FEATURES.map(feature => (
          <div
            key={feature.title}
            className="glow-border group rounded-xl bg-sc-bg-dark p-4 transition-all hover:-translate-y-1 sm:rounded-2xl sm:p-6"
          >
            <div className="mb-4 inline-flex rounded-xl bg-sc-purple/10 p-3 text-sc-purple transition-colors group-hover:bg-sc-purple/20">
              <Glyph>{feature.icon}</Glyph>
            </div>
            <h3 className="mb-2 text-lg font-semibold text-sc-fg">{feature.title}</h3>
            <p className="text-sm leading-relaxed text-sc-fg-muted">{feature.description}</p>
          </div>
        ))}
      </div>
    </section>
  );
}
