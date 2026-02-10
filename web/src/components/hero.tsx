export function Hero() {
  return (
    <section className="relative flex min-h-[85vh] flex-col items-center justify-center overflow-hidden px-6 pt-20 text-center">
      {/* Background gradient orbs */}
      <div className="pointer-events-none absolute inset-0">
        <div className="absolute left-1/4 top-1/4 h-96 w-96 rounded-full bg-sc-purple/10 blur-[120px]" />
        <div className="absolute bottom-1/4 right-1/4 h-96 w-96 rounded-full bg-sc-cyan/8 blur-[120px]" />
        <div className="absolute left-1/2 top-1/2 h-64 w-64 -translate-x-1/2 -translate-y-1/2 rounded-full bg-sc-coral/5 blur-[100px]" />
      </div>

      {/* Content */}
      <div className="relative z-10 max-w-4xl">
        <div className="mb-6 inline-flex items-center gap-2 rounded-full border border-sc-purple/20 bg-sc-bg-highlight/50 px-4 py-1.5 text-sm text-sc-fg-muted">
          <span className="inline-block h-2 w-2 animate-pulse rounded-full bg-sc-success" />
          40+ themes &middot; Powered by Typst
        </div>

        <h1 className="mb-6 text-5xl font-extrabold leading-[1.1] tracking-tight md:text-7xl">
          Markdown to PDF, <span className="gradient-text-shimmer block">made stunning.</span>
        </h1>

        <p className="mx-auto mb-10 max-w-2xl text-lg leading-relaxed text-sc-fg-muted md:text-xl">
          Paste your Markdown. Pick a gorgeous theme. Get a print-ready PDF in seconds. No LaTeX, no
          setup, no suffering.
        </p>

        <div className="flex flex-col items-center justify-center gap-4 sm:flex-row">
          <a
            href="#editor"
            className="group relative inline-flex items-center gap-2 overflow-hidden rounded-xl bg-gradient-to-r from-sc-purple to-sc-coral px-8 py-3.5 text-base font-semibold text-white transition-all hover:-translate-y-0.5 hover:shadow-[0_8px_30px_rgba(225,53,255,0.3)]"
          >
            <span className="relative z-10">Try it now</span>
            <svg
              aria-hidden="true"
              className="relative z-10 h-4 w-4 transition-transform group-hover:translate-x-1"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              strokeWidth={2}
            >
              <path strokeLinecap="round" strokeLinejoin="round" d="M13 7l5 5m0 0l-5 5m5-5H6" />
            </svg>
          </a>
          <a
            href="#themes"
            className="glow-border inline-flex items-center gap-2 rounded-xl px-8 py-3.5 text-base font-semibold text-sc-cyan transition-all hover:-translate-y-0.5"
          >
            View Themes
          </a>
        </div>
      </div>

      {/* Scroll indicator */}
      <div className="absolute bottom-8 left-1/2 -translate-x-1/2 animate-bounce text-sc-fg-dim">
        <svg
          aria-hidden="true"
          className="h-6 w-6"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
          strokeWidth={2}
        >
          <path strokeLinecap="round" strokeLinejoin="round" d="M19 14l-7 7m0 0l-7-7m7 7V3" />
        </svg>
      </div>
    </section>
  );
}
