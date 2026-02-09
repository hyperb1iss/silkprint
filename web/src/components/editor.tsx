'use client';

import { useCallback, useEffect, useRef, useState } from 'react';

import { PdfPreview } from './pdf-preview';
import { ThemeSelector } from './theme-selector';

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

type EngineState =
  | { status: 'loading'; progress: string }
  | { status: 'ready' }
  | { status: 'rendering' }
  | { status: 'error'; message: string };

export function Editor() {
  const [markdown, setMarkdown] = useState(SAMPLE_MARKDOWN);
  const [activeTheme, setActiveTheme] = useState('silk-light');
  const [engineState, setEngineState] = useState<EngineState>({
    status: 'loading',
    progress: 'Downloading SilkPrint engine...',
  });
  const [pdfBytes, setPdfBytes] = useState<Uint8Array | null>(null);
  const [renderError, setRenderError] = useState<string | null>(null);
  const [engineReady, setEngineReady] = useState(false);
  const [loadAttempt, setLoadAttempt] = useState(0);

  const silkprintRef = useRef<typeof import('@/lib/silkprint') | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const renderGenRef = useRef(0);
  const hasRendered = useRef(false);
  // Latest pdfBytes synced from effect — event handlers read from this ref
  // to avoid stale closures from React's async rendering pipeline.
  const pdfBytesRef = useRef<Uint8Array | null>(null);

  // Keep ref in sync with state (useEffect captures committed state correctly)
  useEffect(() => {
    pdfBytesRef.current = pdfBytes;
  }, [pdfBytes]);

  // Auto-load engine on mount (re-runs on retry via loadAttempt)
  // biome-ignore lint/correctness/useExhaustiveDependencies: loadAttempt triggers retry
  useEffect(() => {
    let cancelled = false;
    setEngineReady(false);
    hasRendered.current = false;

    setEngineState({ status: 'loading', progress: 'Downloading SilkPrint engine...' });

    (async () => {
      try {
        const silkprint = await import('@/lib/silkprint');

        if (cancelled) return;
        setEngineState({ status: 'loading', progress: 'Initializing Typst compiler...' });

        // Triggers WASM init (uses the module-level preloaded fetch)
        await silkprint.listThemes();
        if (cancelled) return;

        silkprintRef.current = silkprint;
        setEngineState({ status: 'ready' });
        setEngineReady(true);
      } catch (err) {
        if (cancelled) return;
        const msg = err instanceof Error ? err.message : 'Failed to load engine';
        setEngineState({ status: 'error', message: msg });
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [loadAttempt]);

  // Render effect — fires on engine ready and input changes.
  // Uses a generation counter to discard stale results from superseded renders.
  useEffect(() => {
    if (!engineReady) return;

    const gen = ++renderGenRef.current;
    const delay = hasRendered.current ? 500 : 0;
    hasRendered.current = true;

    const timeout = setTimeout(async () => {
      const silkprint = silkprintRef.current;
      if (!silkprint) return;

      setEngineState({ status: 'rendering' });
      setRenderError(null);

      try {
        const bytes = await silkprint.renderPdf(markdown, activeTheme);
        if (renderGenRef.current !== gen) return;
        setPdfBytes(new Uint8Array(bytes));
      } catch (err) {
        if (renderGenRef.current !== gen) return;
        const msg = err instanceof Error ? err.message : 'Render failed';
        setRenderError(msg);
      }

      if (renderGenRef.current === gen) {
        setEngineState({ status: 'ready' });
      }
    }, delay);

    return () => {
      clearTimeout(timeout);
    };
  }, [engineReady, markdown, activeTheme]);

  // Download reads from ref (synced via useEffect) to dodge stale closures
  const handleDownload = useCallback(() => {
    const bytes = pdfBytesRef.current;
    if (!bytes || bytes.length === 0) return;
    const blob = new Blob([bytes.buffer as ArrayBuffer], { type: 'application/pdf' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'silkprint-document.pdf';
    a.style.display = 'none';
    document.body.appendChild(a);
    a.click();
    setTimeout(() => {
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    }, 1000);
  }, []);

  // File upload handler
  const handleFileUpload = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => {
      if (typeof reader.result === 'string') {
        setMarkdown(reader.result);
      }
    };
    reader.readAsText(file);
    e.target.value = '';
  }, []);

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
      <ThemeSelector activeTheme={activeTheme} onSelect={setActiveTheme} />

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
            <div className="flex items-center gap-2">
              <button
                type="button"
                onClick={() => fileInputRef.current?.click()}
                className="flex items-center gap-1 rounded-md bg-sc-bg-highlight px-2 py-1 text-xs text-sc-fg-muted transition-colors hover:bg-sc-bg-surface hover:text-sc-fg"
              >
                <svg
                  aria-hidden="true"
                  className="h-3 w-3"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                  strokeWidth={2}
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
                  />
                </svg>
                Upload
              </button>
              <input
                ref={fileInputRef}
                type="file"
                accept=".md,.markdown,.txt,.mdx"
                onChange={handleFileUpload}
                className="hidden"
              />
              <span className="font-mono text-xs text-sc-fg-dim">Markdown</span>
            </div>
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
              disabled={!pdfBytes || pdfBytes.length === 0}
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
                  onClick={() => setLoadAttempt(n => n + 1)}
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
                {pdfBytes && pdfBytes.length > 0 ? (
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
