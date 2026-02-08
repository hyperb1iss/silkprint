import type { InitOutput } from './wasm/silkprint_wasm';

let wasmModule: InitOutput | null = null;
let initPromise: Promise<InitOutput> | null = null;

const FONT_FILES = [
  'inter/Inter-Regular.ttf',
  'inter/Inter-Medium.ttf',
  'inter/Inter-SemiBold.ttf',
  'inter/Inter-Bold.ttf',
  'source-serif/SourceSerif4-Regular.ttf',
  'source-serif/SourceSerif4-Italic.ttf',
  'source-serif/SourceSerif4-SemiBold.ttf',
  'source-serif/SourceSerif4-Bold.ttf',
  'jetbrains-mono/JetBrainsMono-Regular.ttf',
  'jetbrains-mono/JetBrainsMono-Italic.ttf',
  'jetbrains-mono/JetBrainsMono-Bold.ttf',
  'jetbrains-mono/JetBrainsMono-BoldItalic.ttf',
];

// Pre-warm: start all fetches immediately on module load.
// WASM + 12 fonts download in parallel over HTTP/2.
const wasmFetchPromise: Promise<Response> | null =
  typeof window !== 'undefined' ? fetch('/wasm/silkprint_wasm_bg.wasm') : null;

const fontFetchPromises: Promise<ArrayBuffer>[] =
  typeof window !== 'undefined'
    ? FONT_FILES.map(f => fetch(`/fonts/${f}`).then(r => r.arrayBuffer()))
    : [];

/**
 * Lazily initialize the SilkPrint WASM module and register fonts.
 *
 * Uses module-level preloaded fetches so WASM compilation and font
 * downloads race in parallel from first import.
 */
async function ensureInit(): Promise<InitOutput> {
  if (wasmModule) return wasmModule;

  if (!initPromise) {
    initPromise = (async () => {
      const [wasm, fontBuffers] = await Promise.all([
        import('./wasm/silkprint_wasm'),
        Promise.all(fontFetchPromises),
      ]);

      const source = wasmFetchPromise ?? fetch('/wasm/silkprint_wasm_bg.wasm');
      const output = await wasm.default(await source);

      // Register all fonts before first render
      for (const buf of fontBuffers) {
        wasm.register_font(new Uint8Array(buf));
      }

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
