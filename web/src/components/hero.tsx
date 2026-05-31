import { TerminalFrame } from './terminal-frame';

export function Hero() {
  return (
    <section className="relative flex flex-col items-center overflow-hidden px-4 pb-12 pt-28 text-center md:px-6 md:pt-36">
      {/* Background gradient orbs */}
      <div className="pointer-events-none absolute inset-0">
        <div className="absolute left-1/4 top-20 h-[28rem] w-[28rem] rounded-full bg-sc-purple/15 blur-[140px]" />
        <div className="absolute right-1/5 top-1/3 h-96 w-96 rounded-full bg-sc-cyan/10 blur-[140px]" />
        <div className="absolute left-1/2 top-1/2 h-72 w-72 -translate-x-1/2 rounded-full bg-sc-coral/8 blur-[120px]" />
      </div>

      <div className="relative z-10 flex max-w-4xl flex-col items-center">
        <div className="mb-6 inline-flex items-center gap-2 rounded-full border border-sc-purple/20 bg-sc-bg-highlight/50 px-4 py-1.5 text-sm text-sc-fg-muted">
          <span className="inline-block h-2 w-2 animate-pulse rounded-full bg-sc-success" />
          Terminal reader + PDF &middot; 40 themes &middot; built in Rust
        </div>

        <h1 className="mb-6 text-4xl font-extrabold leading-[1.05] tracking-tight sm:text-5xl md:text-7xl">
          Read Markdown <span className="gradient-text-shimmer">beautifully.</span>
        </h1>

        <p className="mx-auto mb-9 max-w-2xl text-base leading-relaxed text-sc-fg-muted sm:text-lg md:text-xl">
          A gorgeous terminal reader and publication-ready PDFs from one themed engine — inline
          images, mermaid diagrams, live theme switching, and 40 themes that drive both.
        </p>

        <div className="mb-14 flex flex-col items-center justify-center gap-4 sm:flex-row">
          <a
            href="#reader"
            className="group relative inline-flex items-center gap-2 overflow-hidden rounded-xl bg-gradient-to-r from-sc-purple to-sc-coral px-8 py-3.5 text-base font-semibold text-white transition-all hover:-translate-y-0.5 hover:shadow-[0_8px_30px_rgba(225,53,255,0.35)]"
          >
            <span className="relative z-10">See the reader</span>
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
            href="#pdf"
            className="glow-border inline-flex items-center gap-2 rounded-xl px-8 py-3.5 text-base font-semibold text-sc-cyan transition-all hover:-translate-y-0.5"
          >
            Try the PDF editor
          </a>
        </div>
      </div>

      {/* The animated reader, framed as a real terminal */}
      <div className="relative z-10 w-full max-w-5xl">
        <TerminalFrame label="silkprint read README.md" glow>
          {/* biome-ignore lint/performance/noImgElement: animated GIF must stay unoptimized */}
          <img
            src="/reader/demo.gif"
            alt="SilkPrint terminal reader: scrolling, mermaid diagrams, live theme switching across the SilkCircuit family, and search"
            className="block w-full"
            width={1100}
            height={812}
          />
        </TerminalFrame>
        <p className="mt-4 text-center text-sm text-sc-fg-dim">
          Scroll &middot; mermaid diagrams &middot; live theme switching &middot; search — all in
          your terminal
        </p>
      </div>
    </section>
  );
}
