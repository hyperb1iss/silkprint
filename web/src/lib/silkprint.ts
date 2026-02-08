import type { InitOutput } from './wasm/silkprint_wasm';

let wasmModule: InitOutput | null = null;
let initPromise: Promise<InitOutput> | null = null;

// Start downloading the WASM binary as soon as this module loads.
// The dynamic import of this file is triggered on component mount,
// so the ~41MB fetch races with Typst compiler initialization.
const wasmFetchPromise: Promise<Response> | null =
  typeof window !== 'undefined' ? fetch('/wasm/silkprint_wasm_bg.wasm') : null;

/**
 * Lazily initialize the SilkPrint WASM module.
 *
 * Uses the module-level preloaded fetch if available, giving
 * streaming WASM compilation a head start on the binary download.
 */
async function ensureInit(): Promise<InitOutput> {
  if (wasmModule) return wasmModule;

  if (!initPromise) {
    initPromise = (async () => {
      const wasm = await import('./wasm/silkprint_wasm');
      const source = wasmFetchPromise ?? fetch('/wasm/silkprint_wasm_bg.wasm');
      const output = await wasm.default(await source);
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
  print_safe: boolean;
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
