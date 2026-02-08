'use client';

import { useState } from 'react';

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

export function Editor() {
  const [markdown, setMarkdown] = useState(SAMPLE_MARKDOWN);
  const [activeTheme, setActiveTheme] = useState('silk-light');

  const themes = [
    { id: 'silk-light', name: 'Silk Light', variant: 'light' },
    { id: 'silk-dark', name: 'Silk Dark', variant: 'dark' },
    { id: 'silkcircuit-neon', name: 'SilkCircuit Neon', variant: 'dark' },
    { id: 'manuscript', name: 'Manuscript', variant: 'light' },
    { id: 'nord', name: 'Nord', variant: 'dark' },
    { id: 'dracula', name: 'Dracula', variant: 'dark' },
    { id: 'catppuccin-mocha', name: 'Catppuccin Mocha', variant: 'dark' },
    { id: 'tokyo-night', name: 'Tokyo Night', variant: 'dark' },
  ];

  return (
    <section id="editor" className="mx-auto max-w-7xl px-6 py-20">
      <div className="mb-10 text-center">
        <h2 className="mb-3 text-3xl font-bold tracking-tight md:text-4xl">
          <span className="gradient-text">Live Editor</span>
        </h2>
        <p className="text-sc-fg-muted">Paste your Markdown, pick a theme, download your PDF.</p>
      </div>

      {/* Theme selector */}
      <div className="mb-6 flex flex-wrap items-center justify-center gap-2">
        {themes.map(theme => (
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
            </div>
            <button
              type="button"
              className="group flex items-center gap-1.5 rounded-lg bg-gradient-to-r from-sc-purple to-sc-coral px-3 py-1.5 text-xs font-semibold text-white transition-all hover:-translate-y-0.5 hover:shadow-[0_4px_15px_rgba(225,53,255,0.3)]"
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

          {/* Mock PDF preview */}
          <div className="editor-scrollbar h-[500px] overflow-y-auto p-6">
            <PreviewPane markdown={markdown} theme={activeTheme} />
          </div>
        </div>
      </div>
    </section>
  );
}

function PreviewPane({ markdown, theme }: { markdown: string; theme: string }) {
  const isLight = theme === 'silk-light' || theme === 'manuscript';

  return (
    <div
      className={`mx-auto min-h-full max-w-md rounded-lg p-8 shadow-2xl transition-colors ${
        isLight ? 'bg-white text-gray-900' : 'bg-[#1e1e2e] text-gray-100'
      }`}
      style={{
        fontFamily: "Georgia, 'Times New Roman', serif",
        fontSize: '11px',
        lineHeight: '1.6',
      }}
    >
      {markdown.split('\n').map((line, i) => {
        if (line.startsWith('# ')) {
          return (
            <h1
              key={i}
              className={`mb-3 text-xl font-bold ${isLight ? 'text-gray-900' : 'text-white'}`}
              style={{ fontFamily: 'Inter, sans-serif' }}
            >
              {line.slice(2)}
            </h1>
          );
        }
        if (line.startsWith('## ')) {
          return (
            <h2
              key={i}
              className={`mb-2 mt-4 text-base font-bold ${
                isLight ? 'text-gray-800' : 'text-gray-100'
              }`}
              style={{ fontFamily: 'Inter, sans-serif' }}
            >
              {line.slice(3)}
            </h2>
          );
        }
        if (line.startsWith('- ')) {
          return (
            <div key={i} className="mb-1 flex gap-2 pl-3">
              <span className="text-sc-purple">&#x2022;</span>
              <span>{renderInline(line.slice(2), isLight)}</span>
            </div>
          );
        }
        if (line.startsWith('```')) {
          return null;
        }
        if (line.startsWith('|')) {
          return (
            <div
              key={i}
              className={`font-mono text-[10px] ${
                isLight ? 'text-gray-600 even:bg-gray-50' : 'text-gray-400 even:bg-white/5'
              }`}
            >
              {line}
            </div>
          );
        }
        if (line.startsWith('---')) {
          return (
            <hr
              key={i}
              className={`my-4 border-t ${isLight ? 'border-gray-200' : 'border-gray-700'}`}
            />
          );
        }
        if (line.startsWith('> ')) {
          return (
            <div
              key={i}
              className={`my-1 border-l-2 pl-3 text-[10px] italic ${
                isLight ? 'border-blue-300 text-gray-600' : 'border-blue-400 text-gray-400'
              }`}
            >
              {line.slice(2)}
            </div>
          );
        }
        if (line.trim() === '') {
          return <div key={i} className="h-2" />;
        }
        return (
          <p key={i} className="mb-1">
            {renderInline(line, isLight)}
          </p>
        );
      })}
    </div>
  );
}

function renderInline(text: string, isLight: boolean) {
  const parts = text.split(/(\*\*.*?\*\*|_.*?_|`.*?`)/g);
  return parts.map((part, i) => {
    if (part.startsWith('**') && part.endsWith('**')) {
      return (
        <strong key={i} className="font-bold">
          {part.slice(2, -2)}
        </strong>
      );
    }
    if (part.startsWith('_') && part.endsWith('_')) {
      return (
        <em key={i} className="italic">
          {part.slice(1, -1)}
        </em>
      );
    }
    if (part.startsWith('`') && part.endsWith('`')) {
      return (
        <code
          key={i}
          className={`rounded px-1 py-0.5 font-mono text-[10px] ${
            isLight ? 'bg-gray-100 text-pink-600' : 'bg-white/10 text-pink-300'
          }`}
        >
          {part.slice(1, -1)}
        </code>
      );
    }
    return <span key={i}>{part}</span>;
  });
}
