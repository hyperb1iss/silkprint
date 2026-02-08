import type { InitOutput } from './wasm/silkprint_wasm';

let wasmModule: InitOutput | null = null;
let initPromise: Promise<InitOutput> | null = null;

/**
 * Lazily initialize the SilkPrint WASM module.
 *
 * Fetches the ~41MB WASM binary from /wasm/ on first call,
 * then reuses the cached module for subsequent calls.
 */
async function ensureInit(): Promise<InitOutput> {
  if (wasmModule) return wasmModule;

  if (!initPromise) {
    initPromise = (async () => {
      const wasm = await import('./wasm/silkprint_wasm');
      const output = await wasm.default('/wasm/silkprint_wasm_bg.wasm');
      wasmModule = output;
      return output;
    })();
  }

  return initPromise;
}

/**
 * Render markdown to PDF bytes using a built-in theme.
 */
export async function renderPdf(markdown: string, theme: string): Promise<Uint8Array> {
  await ensureInit();
  const wasm = await import('./wasm/silkprint_wasm');
  return wasm.render_pdf(markdown, theme);
}

/**
 * Render markdown to PDF bytes with explicit paper size.
 */
export async function renderPdfWithOptions(
  markdown: string,
  theme: string,
  paper: string
): Promise<Uint8Array> {
  await ensureInit();
  const wasm = await import('./wasm/silkprint_wasm');
  return wasm.render_pdf_with_options(markdown, theme, paper);
}

/**
 * Render markdown to Typst source (for debugging).
 */
export async function renderToTypst(markdown: string, theme: string): Promise<string> {
  await ensureInit();
  const wasm = await import('./wasm/silkprint_wasm');
  return wasm.render_to_typst(markdown, theme);
}

export interface ThemeInfo {
  name: string;
  variant: string;
  description: string;
  printSafe: boolean;
}

/**
 * Get all available theme names.
 */
export async function listThemes(): Promise<string[]> {
  await ensureInit();
  const wasm = await import('./wasm/silkprint_wasm');
  return JSON.parse(wasm.list_themes_json()) as string[];
}

/**
 * Get detailed theme metadata.
 */
export async function listThemesDetailed(): Promise<ThemeInfo[]> {
  await ensureInit();
  const wasm = await import('./wasm/silkprint_wasm');
  return JSON.parse(wasm.list_themes_detailed()) as ThemeInfo[];
}
