/* tslint:disable */
/* eslint-disable */

/**
 * Get all available theme names as a JavaScript array.
 */
export function list_themes(): any;

/**
 * Get detailed theme metadata as JSON.
 *
 * Returns an array of `{name, variant, description, print_safe}` objects.
 */
export function list_themes_detailed(): string;

/**
 * Get all available theme names as a JSON array string.
 *
 * Returns `["silk-light","silk-dark","silkcircuit-neon",...]`
 */
export function list_themes_json(): string;

/**
 * Get detailed theme metadata as structured JavaScript objects.
 */
export function list_themes_structured(): any;

/**
 * Register a font file for use by the renderer.
 *
 * Call once per font file after WASM init, before the first render.
 * Accepts raw TTF/OTF bytes.
 */
export function register_font(data: Uint8Array): void;

/**
 * Render markdown to PDF bytes using a built-in theme.
 *
 * Returns the raw PDF as a `Uint8Array` in JavaScript.
 */
export function render_pdf(markdown: string, theme_name: string): Uint8Array;

/**
 * Render markdown to PDF bytes with explicit paper size.
 *
 * Paper sizes: "a4", "letter", "a5", "legal" (case-insensitive).
 */
export function render_pdf_with_options(markdown: string, theme_name: string, paper: string): Uint8Array;

/**
 * Render markdown to Typst source markup (for debugging/inspection).
 */
export function render_to_typst(markdown: string, theme_name: string): string;

/**
 * Clear all previously registered fonts.
 *
 * Useful for hot reload flows or when swapping font sets at runtime.
 */
export function reset_fonts(): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly list_themes: () => [number, number, number];
    readonly list_themes_detailed: () => [number, number];
    readonly list_themes_json: () => [number, number];
    readonly list_themes_structured: () => [number, number, number];
    readonly register_font: (a: number, b: number) => void;
    readonly render_pdf: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly render_pdf_with_options: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number, number];
    readonly render_to_typst: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly reset_fonts: () => void;
    readonly qcms_profile_precache_output_transform: (a: number) => void;
    readonly lut_inverse_interp16: (a: number, b: number, c: number) => number;
    readonly qcms_transform_data_bgra_out_lut_precache: (a: number, b: number, c: number, d: number) => void;
    readonly qcms_transform_data_rgba_out_lut_precache: (a: number, b: number, c: number, d: number) => void;
    readonly qcms_transform_data_bgra_out_lut: (a: number, b: number, c: number, d: number) => void;
    readonly qcms_transform_data_rgba_out_lut: (a: number, b: number, c: number, d: number) => void;
    readonly qcms_transform_data_rgb_out_lut_precache: (a: number, b: number, c: number, d: number) => void;
    readonly qcms_transform_data_rgb_out_lut: (a: number, b: number, c: number, d: number) => void;
    readonly lut_interp_linear16: (a: number, b: number, c: number) => number;
    readonly qcms_profile_is_bogus: (a: number) => number;
    readonly qcms_transform_release: (a: number) => void;
    readonly qcms_white_point_sRGB: (a: number) => void;
    readonly qcms_enable_iccv4: () => void;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
