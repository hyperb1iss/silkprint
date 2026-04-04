import type { InitOutput } from './wasm/silkprint_wasm';

let wasmModule: InitOutput | null = null;
let initPromise: Promise<InitOutput> | null = null;

const BASE_PATH = process.env.NEXT_PUBLIC_BASE_PATH ?? '';

const FONT_FILES = [
  { path: 'inter/Inter-Regular.ttf', required: true },
  { path: 'inter/Inter-Medium.ttf', required: true },
  { path: 'inter/Inter-SemiBold.ttf', required: true },
  { path: 'inter/Inter-Bold.ttf', required: true },
  { path: 'source-serif/SourceSerif4-Regular.ttf', required: true },
  { path: 'source-serif/SourceSerif4-Italic.ttf', required: true },
  { path: 'source-serif/SourceSerif4-SemiBold.ttf', required: true },
  { path: 'source-serif/SourceSerif4-Bold.ttf', required: true },
  { path: 'jetbrains-mono/JetBrainsMono-Regular.ttf', required: true },
  { path: 'jetbrains-mono/JetBrainsMono-Italic.ttf', required: true },
  { path: 'jetbrains-mono/JetBrainsMono-Bold.ttf', required: true },
  { path: 'jetbrains-mono/JetBrainsMono-BoldItalic.ttf', required: true },
  { path: 'noto-emoji/NotoColorEmoji.ttf', required: false },
] as const;

async function fetchRequired(url: string): Promise<Response> {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`Failed to fetch ${url}: ${response.status} ${response.statusText}`);
  }

  return response;
}

async function fetchArrayBuffer(url: string): Promise<ArrayBuffer> {
  const response = await fetchRequired(url);
  return response.arrayBuffer();
}

async function fetchOptionalArrayBuffer(url: string): Promise<ArrayBuffer | null> {
  const response = await fetch(url);
  if (!response.ok) {
    return null;
  }

  return response.arrayBuffer();
}

// Pre-warm: start all fetches immediately on module load.
// WASM + bundled fonts download in parallel over HTTP/2.
const wasmFetchPromise: Promise<Response> | null =
  typeof window !== 'undefined' ? fetchRequired(`${BASE_PATH}/wasm/silkprint_wasm_bg.wasm`) : null;

const fontFetchPromises: Promise<ArrayBuffer | null>[] =
  typeof window !== 'undefined'
    ? FONT_FILES.map(async ({ path, required }) => {
        const url = `${BASE_PATH}/fonts/${path}`;
        const buffer = required ? await fetchArrayBuffer(url) : await fetchOptionalArrayBuffer(url);
        return buffer;
      })
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
      try {
        const [wasm, fontBuffers] = await Promise.all([
          import('./wasm/silkprint_wasm'),
          Promise.all(fontFetchPromises),
        ]);

        const source =
          wasmFetchPromise ?? fetchRequired(`${BASE_PATH}/wasm/silkprint_wasm_bg.wasm`);
        const output = await wasm.default({ module_or_path: await source });

        // Reset first so retries or hot reloads don't accumulate duplicate blobs.
        wasm.reset_fonts();

        for (const buf of fontBuffers) {
          if (!buf) continue;
          wasm.register_font(new Uint8Array(buf));
        }

        wasmModule = output;
        return output;
      } catch (error) {
        initPromise = null;
        wasmModule = null;
        throw error;
      }
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
  family: string;
  printSafe: boolean;
}

/**
 * Get all available theme names.
 */
export async function listThemes(): Promise<string[]> {
  await ensureInit();
  const wasm = await import('./wasm/silkprint_wasm');
  return wasm.list_themes() as string[];
}

/**
 * Get detailed theme metadata.
 */
export async function listThemesDetailed(): Promise<ThemeInfo[]> {
  await ensureInit();
  const wasm = await import('./wasm/silkprint_wasm');
  return wasm.list_themes_structured() as ThemeInfo[];
}
