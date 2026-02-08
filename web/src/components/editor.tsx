'use client';

import { useCallback, useEffect, useRef, useState } from 'react';

import { PdfPreview } from './pdf-preview';

const SAMPLE_MARKDOWN = `# Welcome to SilkPrint

Transform your **Markdown** into _stunning_ PDFs with beautiful themes.

## Features

- 40+ gorgeous built-in themes
- Print-safe color validation
- Syntax highlighting with custom palettes
- Math, tables, alerts, and more

## Code Example

\`\`\`rust
fn main() {
    println!("Hello, SilkPrint!");
}
\`\`\`

## Table

| Theme | Variant | Print Safe |
|-------|---------|------------|
| Silk Light | Light | Yes |
| SilkCircuit Neon | Dark | No |
| Manuscript | Light | Yes |
| Nord | Dark | No |

> [!NOTE]
> SilkPrint uses Typst under the hood for
> pixel-perfect PDF rendering.

---

*Beautiful documents, effortlessly.*
`;

const THEMES = [
  { id: 'silk-light', name: 'Silk Light', variant: 'light' },
  { id: 'silk-dark', name: 'Silk Dark', variant: 'dark' },
  { id: 'silkcircuit-neon', name: 'SilkCircuit Neon', variant: 'dark' },
  { id: 'manuscript', name: 'Manuscript', variant: 'light' },
  { id: 'nord', name: 'Nord', variant: 'dark' },
  { id: 'dracula', name: 'Dracula', variant: 'dark' },
  { id: 'catppuccin-mocha', name: 'Catppuccin Mocha', variant: 'dark' },
  { id: 'tokyo-night', name: 'Tokyo Night', variant: 'dark' },
];

type EngineState =
  | { status: 'idle' }
  | { status: 'loading'; progress: string }
  | { status: 'ready' }
  | { status: 'rendering' }
  | { status: 'error'; message: string };

export function Editor() {
  const [markdown, setMarkdown] = useState(SAMPLE_MARKDOWN);
  const [activeTheme, setActiveTheme] = useState('silk-light');
  const [engineState, setEngineState] = useState<EngineState>({ status: 'idle' });
  const [pdfBytes, setPdfBytes] = useState<Uint8Array | null>(null);
  const [renderError, setRenderError] = useState<string | null>(null);
  const renderTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const silkprintRef = useRef<typeof import('@/lib/silkprint') | null>(null);

  // Load the WASM engine lazily
  const loadEngine = useCallback(async () => {
    if (engineState.status === 'loading' || engineState.status === 'ready') return;

    setEngineState({ status: 'loading', progress: 'Downloading SilkPrint engine...' });

    try {
      const silkprint = await import('@/lib/silkprint');
      setEngineState({ status: 'loading', progress: 'Initializing Typst compiler...' });

      // Trigger WASM initialization by listing themes (lightweight call)
      await silkprint.listThemes();
      silkprintRef.current = silkprint;
      setEngineState({ status: 'ready' });
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to load engine';
      setEngineState({ status: 'error', message: msg });
    }
  }, [engineState.status]);

  // Render PDF when markdown or theme changes (debounced)
  const triggerRender = useCallback(async (md: string, theme: string) => {
    const silkprint = silkprintRef.current;
    if (!silkprint) return;

    setEngineState({ status: 'rendering' });
    setRenderError(null);

    try {
      const bytes = await silkprint.renderPdf(md, theme);
      // Copy out of WASM linear memory — the backing ArrayBuffer gets
      // detached on subsequent renders, so we need an independent copy.
      setPdfBytes(new Uint8Array(bytes));
      setEngineState({ status: 'ready' });
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Render failed';
      setRenderError(msg);
      setEngineState({ status: 'ready' });
    }
  }, []);

  // Debounced render effect
  useEffect(() => {
    if (engineState.status !== 'ready' && engineState.status !== 'rendering') return;

    if (renderTimeoutRef.current) {
      clearTimeout(renderTimeoutRef.current);
    }

    renderTimeoutRef.current = setTimeout(() => {
      triggerRender(markdown, activeTheme);
    }, 500);

    return () => {
      if (renderTimeoutRef.current) {
        clearTimeout(renderTimeoutRef.current);
      }
    };
  }, [markdown, activeTheme, engineState.status, triggerRender]);

  // Download PDF
  const handleDownload = useCallback(() => {
    if (!pdfBytes) return;
    // Uint8Array is a valid BlobPart at runtime; the TS generic mismatch
    // (ArrayBufferLike vs ArrayBuffer) is a strict-mode false positive.
    const blob = new Blob([pdfBytes as unknown as Uint8Array<ArrayBuffer>], {
      type: 'application/pdf',
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'silkprint-document.pdf';
    a.click();
    URL.revokeObjectURL(url);
  }, [pdfBytes]);

  return (
    <section id="editor" className="mx-auto max-w-7xl px-6 py-20">
      <div className="mb-10 text-center">
        <h2 className="mb-3 text-3xl font-bold tracking-tight md:text-4xl">
          <span className="gradient-text">Live Editor</span>
        </h2>
        <p className="text-sc-fg-muted">
          Paste your Markdown, pick a theme, get a real PDF — rendered entirely in your browser.
        </p>
      </div>

      {/* Theme selector */}
      <div className="mb-6 flex flex-wrap items-center justify-center gap-2">
        {THEMES.map(theme => (
          <button
            type="button"
            key={theme.id}
            onClick={() => setActiveTheme(theme.id)}
            className={`rounded-lg px-3 py-1.5 text-sm font-medium transition-all ${
              activeTheme === theme.id
                ? 'bg-sc-purple/20 text-sc-purple shadow-[0_0_12px_rgba(225,53,255,0.2)]'
                : 'bg-sc-bg-highlight text-sc-fg-muted hover:bg-sc-bg-surface hover:text-sc-fg'
            }`}
          >
            <span
              className={`mr-1.5 inline-block h-2 w-2 rounded-full ${
                theme.variant === 'light' ? 'bg-amber-200' : 'bg-indigo-400'
              }`}
            />
            {theme.name}
          </button>
        ))}
      </div>

      {/* Editor / Preview split */}
      <div className="glow-border grid grid-cols-1 overflow-hidden rounded-2xl bg-sc-bg-dark lg:grid-cols-2">
        {/* Markdown input */}
        <div className="flex flex-col border-b border-sc-cyan/10 lg:border-b-0 lg:border-r">
          <div className="flex items-center justify-between border-b border-sc-cyan/10 px-4 py-2.5">
            <div className="flex items-center gap-2">
              <div className="flex gap-1.5">
                <span className="h-3 w-3 rounded-full bg-sc-error/60" />
                <span className="h-3 w-3 rounded-full bg-sc-warning/60" />
                <span className="h-3 w-3 rounded-full bg-sc-success/60" />
              </div>
              <span className="text-xs font-medium text-sc-fg-dim">document.md</span>
            </div>
            <span className="font-mono text-xs text-sc-fg-dim">Markdown</span>
          </div>
          <textarea
            value={markdown}
            onChange={e => setMarkdown(e.target.value)}
            className="editor-scrollbar h-[500px] w-full resize-none bg-transparent p-4 font-mono text-sm leading-relaxed text-sc-fg placeholder:text-sc-fg-dim focus:outline-none"
            placeholder="Paste your Markdown here..."
            spellCheck={false}
          />
        </div>

        {/* Preview panel */}
        <div className="flex flex-col">
          <div className="flex items-center justify-between border-b border-sc-cyan/10 px-4 py-2.5">
            <div className="flex items-center gap-2">
              <span className="text-xs font-medium text-sc-fg-dim">Preview</span>
              <span className="rounded bg-sc-purple/15 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wider text-sc-purple">
                {activeTheme}
              </span>
              {engineState.status === 'rendering' && (
                <span className="animate-pulse rounded bg-sc-cyan/15 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wider text-sc-cyan">
                  Rendering...
                </span>
              )}
            </div>
            <button
              type="button"
              onClick={handleDownload}
              disabled={!pdfBytes}
              className="group flex items-center gap-1.5 rounded-lg bg-gradient-to-r from-sc-purple to-sc-coral px-3 py-1.5 text-xs font-semibold text-white transition-all hover:-translate-y-0.5 hover:shadow-[0_4px_15px_rgba(225,53,255,0.3)] disabled:cursor-not-allowed disabled:opacity-40 disabled:hover:translate-y-0 disabled:hover:shadow-none"
            >
              <svg
                aria-hidden="true"
                className="h-3.5 w-3.5"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                strokeWidth={2}
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
                />
              </svg>
              Download PDF
            </button>
          </div>

          {/* Preview content */}
          <div className="editor-scrollbar h-[500px] overflow-y-auto">
            {engineState.status === 'idle' && (
              <div className="flex h-full flex-col items-center justify-center gap-4 p-8">
                <div className="text-center">
                  <p className="mb-4 text-sm text-sc-fg-muted">
                    Real PDF rendering powered by SilkPrint + Typst, running entirely in your
                    browser via WebAssembly.
                  </p>
                  <button
                    type="button"
                    onClick={loadEngine}
                    className="group relative rounded-xl bg-gradient-to-r from-sc-purple to-sc-coral px-6 py-3 text-sm font-semibold text-white transition-all hover:-translate-y-0.5 hover:shadow-[0_6px_20px_rgba(225,53,255,0.3)]"
                  >
                    Load SilkPrint Engine
                    <span className="ml-2 text-xs opacity-70">(~22 MB)</span>
                  </button>
                </div>
              </div>
            )}

            {engineState.status === 'loading' && (
              <div className="flex h-full flex-col items-center justify-center gap-4 p-8">
                <LoadingSpinner />
                <p className="text-sm text-sc-fg-muted">{engineState.progress}</p>
              </div>
            )}

            {engineState.status === 'error' && (
              <div className="flex h-full flex-col items-center justify-center gap-4 p-8">
                <p className="text-sm text-sc-error">Failed to load engine</p>
                <p className="max-w-sm text-center text-xs text-sc-fg-dim">{engineState.message}</p>
                <button
                  type="button"
                  onClick={() => {
                    setEngineState({ status: 'idle' });
                    silkprintRef.current = null;
                  }}
                  className="rounded-lg bg-sc-bg-highlight px-4 py-2 text-xs text-sc-fg-muted hover:text-sc-fg"
                >
                  Retry
                </button>
              </div>
            )}

            {(engineState.status === 'ready' || engineState.status === 'rendering') && (
              <>
                {renderError && (
                  <div className="border-b border-sc-error/20 bg-sc-error/10 px-4 py-2 text-xs text-sc-error">
                    {renderError}
                  </div>
                )}
                {pdfBytes ? (
                  <PdfPreview pdfBytes={pdfBytes} className="p-4" />
                ) : (
                  <div className="flex h-full items-center justify-center p-8">
                    <LoadingSpinner />
                  </div>
                )}
              </>
            )}
          </div>
        </div>
      </div>
    </section>
  );
}

function LoadingSpinner() {
  return (
    <div className="relative h-8 w-8">
      <div className="absolute inset-0 animate-spin rounded-full border-2 border-sc-purple/20 border-t-sc-purple" />
    </div>
  );
}
