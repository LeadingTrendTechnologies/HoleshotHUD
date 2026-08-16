/* tslint:disable */
/* eslint-disable */

export class Preview {
    free(): void;
    [Symbol.dispose](): void;
    active_widget(): string;
    frame(width: number, height: number): Uint8Array;
    get_bool(key: string): boolean;
    get_field(key: string): string;
    get_int(key: string): number;
    hover_cursor(nx: number, ny: number, width: number, height: number): string;
    constructor();
    pointer_down(nx: number, ny: number, width: number, height: number): void;
    pointer_move(nx: number, ny: number, width: number, height: number): void;
    pointer_up(): void;
    select_widget(name: string): void;
    set_bool(key: string, on: boolean): void;
    set_field(key: string, value: string): void;
    set_int(key: string, value: number): void;
    set_widget(name: string, on: boolean): void;
    snap_widget(align: string): void;
    tick(dt: number): void;
    widget_on(name: string): boolean;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_preview_free: (a: number, b: number) => void;
    readonly preview_active_widget: (a: number) => [number, number];
    readonly preview_frame: (a: number, b: number, c: number) => [number, number];
    readonly preview_get_bool: (a: number, b: number, c: number) => number;
    readonly preview_get_field: (a: number, b: number, c: number) => [number, number];
    readonly preview_get_int: (a: number, b: number, c: number) => number;
    readonly preview_hover_cursor: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly preview_new: () => [number, number, number];
    readonly preview_pointer_down: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly preview_pointer_move: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly preview_pointer_up: (a: number) => void;
    readonly preview_select_widget: (a: number, b: number, c: number) => void;
    readonly preview_set_bool: (a: number, b: number, c: number, d: number) => void;
    readonly preview_set_field: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly preview_set_int: (a: number, b: number, c: number, d: number) => void;
    readonly preview_set_widget: (a: number, b: number, c: number, d: number) => void;
    readonly preview_snap_widget: (a: number, b: number, c: number) => void;
    readonly preview_tick: (a: number, b: number) => void;
    readonly preview_widget_on: (a: number, b: number, c: number) => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __externref_table_dealloc: (a: number) => void;
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
