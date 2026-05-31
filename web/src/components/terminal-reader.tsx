import { TerminalFrame } from './terminal-frame';

const SHOTS = [
  {
    src: '/reader/mermaid.png',
    label: 'silkprint read docs/demo.md',
    title: 'Diagrams, tables & alerts — inline',
    blurb:
      'Mermaid diagrams rasterized right in the flow, GitHub-style alerts, and striped tables.',
  },
  {
    src: '/reader/picker.png',
    label: 't  —  theme picker',
    title: 'Live theme switching',
    blurb: 'Press t and preview any of 40 themes instantly. No restart, no config edit.',
  },
  {
    src: '/reader/search.png',
    label: '/theme',
    title: 'Search & navigate',
    blurb: 'Incremental search with highlighted matches, an outline sidebar, and link following.',
  },
  {
    src: '/reader/oneshot.png',
    label: 'silkprint read README.md | less -R',
    title: 'Pipe-friendly one-shot',
    blurb: 'Styled ANSI when piped, with graceful fallbacks for images and glyphs.',
  },
];

const THEMES = [
  { src: '/reader/theme-neon.png', name: 'neon' },
  { src: '/reader/theme-dawn.png', name: 'dawn' },
  { src: '/reader/theme-vibrant.png', name: 'vibrant' },
  { src: '/reader/theme-soft.png', name: 'soft' },
  { src: '/reader/theme-glow.png', name: 'glow' },
];

const CAPABILITIES = [
  { dot: 'bg-sc-purple', t: 'Inline images', d: 'Kitty, iTerm2 & Sixel, with halfblock fallback' },
  { dot: 'bg-sc-cyan', t: 'Mermaid diagrams', d: 'rendered to images inline' },
  { dot: 'bg-sc-coral', t: 'Outline + search', d: 'jump by heading, search with highlights' },
  { dot: 'bg-sc-yellow', t: 'Follow links', d: 'open .md in-reader, back/forward history' },
  { dot: 'bg-sc-purple', t: 'Live reload', d: 're-renders the moment you save' },
  { dot: 'bg-sc-cyan', t: 'Graceful degradation', d: 'truecolor → 256 → 16, Nerd Font → ASCII' },
];

const KEYS: [string, string][] = [
  ['j / k', 'scroll'],
  ['Ctrl-d / u', 'half page'],
  ['g g / G', 'top / bottom'],
  ['/  n  N', 'search'],
  ['t', 'themes'],
  ['o', 'outline'],
  ['b / f', 'back / fwd'],
  ['Tab', 'focus'],
  ['q', 'quit'],
];

export function TerminalReader() {
  return (
    <section id="reader" className="relative overflow-hidden px-4 py-16 sm:px-6 md:py-24">
      <div className="pointer-events-none absolute left-1/2 top-0 h-80 w-[40rem] -translate-x-1/2 rounded-full bg-sc-purple/8 blur-[130px]" />

      <div className="relative z-10 mx-auto max-w-7xl">
        <header className="mb-12 text-center md:mb-16">
          <p className="mb-3 font-mono text-sm tracking-wider text-sc-cyan">silkprint read</p>
          <h2 className="mb-3 text-3xl font-bold tracking-tight sm:text-4xl md:text-5xl">
            Your terminal, <span className="gradient-text">upgraded</span>
          </h2>
          <p className="mx-auto max-w-2xl text-sc-fg-muted">
            A scrollable reader built on the same themed engine as the PDF path — so what you read
            in your terminal is exactly what lands on the page.
          </p>
        </header>

        {/* Bento of framed feature shots */}
        <div className="grid gap-6 md:grid-cols-2">
          {SHOTS.map(shot => (
            <figure key={shot.src} className="flex flex-col">
              <TerminalFrame label={shot.label}>
                {/* biome-ignore lint/performance/noImgElement: static export, plain img is fine */}
                <img src={shot.src} alt={shot.title} className="block w-full" loading="lazy" />
              </TerminalFrame>
              <figcaption className="mt-4 px-1">
                <h3 className="text-lg font-semibold text-sc-fg">{shot.title}</h3>
                <p className="mt-1 text-sm leading-relaxed text-sc-fg-muted">{shot.blurb}</p>
              </figcaption>
            </figure>
          ))}
        </div>

        {/* Capabilities */}
        <div className="mt-14 grid grid-cols-1 gap-x-8 gap-y-5 sm:grid-cols-2 lg:grid-cols-3">
          {CAPABILITIES.map(cap => (
            <div key={cap.t} className="flex items-start gap-3">
              <span className={`mt-1.5 h-2 w-2 shrink-0 rounded-full ${cap.dot}`} />
              <p className="text-sm text-sc-fg-muted">
                <span className="font-semibold text-sc-fg">{cap.t}</span> — {cap.d}
              </p>
            </div>
          ))}
        </div>

        {/* Theme strip */}
        <div className="mt-20">
          <div className="mb-6 text-center">
            <h3 className="text-2xl font-bold tracking-tight sm:text-3xl">
              One engine. <span className="gradient-text">Every mood.</span>
            </h3>
            <p className="mx-auto mt-2 max-w-xl text-sm text-sc-fg-muted">
              The same document across the SilkCircuit family — and 35 more themes, each styling the
              reader and the PDF together.
            </p>
          </div>
          <div className="-mx-4 flex snap-x gap-4 overflow-x-auto px-4 pb-4 sm:mx-0 sm:grid sm:grid-cols-3 sm:overflow-visible sm:px-0 lg:grid-cols-5">
            {THEMES.map(theme => (
              <div
                key={theme.name}
                className="glow-border group w-[78vw] shrink-0 snap-center overflow-hidden rounded-lg bg-sc-bg-dark sm:w-auto"
              >
                {/* biome-ignore lint/performance/noImgElement: static export, plain img is fine */}
                <img
                  src={theme.src}
                  alt={`silkcircuit-${theme.name} theme`}
                  className="block w-full"
                  loading="lazy"
                />
                <div className="flex items-center gap-2 px-3 py-2">
                  <span className="h-2 w-2 rounded-full bg-sc-purple" />
                  <span className="font-mono text-xs text-sc-fg-muted group-hover:text-sc-fg">
                    silkcircuit-{theme.name}
                  </span>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Keys */}
        <div className="glass mt-16 rounded-2xl p-6 md:p-8">
          <h3 className="mb-5 text-sm font-semibold uppercase tracking-wider text-sc-fg-dim">
            Keyboard &amp; mouse
          </h3>
          <div className="grid grid-cols-1 gap-x-8 gap-y-3 sm:grid-cols-2 lg:grid-cols-3">
            {KEYS.map(([key, action]) => (
              <div
                key={key}
                className="flex items-center justify-between gap-4 border-b border-white/5 pb-2"
              >
                <kbd className="rounded-md border border-sc-purple/25 bg-sc-bg-surface px-2 py-1 font-mono text-xs text-sc-cyan">
                  {key}
                </kbd>
                <span className="text-sm text-sc-fg-muted">{action}</span>
              </div>
            ))}
          </div>
          <p className="mt-5 text-sm text-sc-fg-dim">
            The mouse scrolls, clicks links and outline entries, and drags to scroll.
          </p>
        </div>
      </div>
    </section>
  );
}
