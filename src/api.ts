import { Channel, invoke } from "@tauri-apps/api/core";
import type { Settings } from "./types";

export function errorMessage(err: unknown) {
  return String(err).replace(/^Error:\s*/i, "").trim() || "Something went wrong.";
}

export const getSettings = () => invoke<Settings>("get_settings");
export const saveSettings = (settings: Settings) => invoke<Settings>("save_settings", { settings });
export const setWindowBlur = (radius: number) => invoke<void>("set_window_blur", { radius });
export const settingsFilePath = () => invoke<string>("settings_file_path");
export const readTextFile = (path: string) => invoke<string>("read_text_file", { path });
export const writeTextFile = (path: string, contents: string) => invoke<void>("write_text_file", { path, contents });
export const setToggleHotkey = (accelerator: string) => invoke<void>("set_toggle_hotkey", { accelerator });
export const pauseToggleHotkey = (paused: boolean) => invoke<void>("pause_toggle_hotkey", { paused });
/** Hides the window and leaves Dispatch in the menu bar. */
export const hideWindow = () => invoke<void>("hide_window");

export interface OpenTerminalOptions {
  cols: number;
  rows: number;
  shell?: string;
  cwd?: string;
  onData: (bytes: Uint8Array) => void;
  onExit: () => void;
}

/** Starts a shell; its output arrives as raw bytes on `onData`. */
export function openTerminal(options: OpenTerminalOptions) {
  const onData = new Channel<ArrayBuffer>();
  onData.onmessage = (buffer) => options.onData(new Uint8Array(buffer));
  const onExit = new Channel<null>();
  onExit.onmessage = () => options.onExit();
  return invoke<string>("open_terminal", {
    cols: options.cols,
    rows: options.rows,
    shell: options.shell || null,
    cwd: options.cwd || null,
    onData,
    onExit,
  });
}

export const writeTerminal = (id: string, data: string) => invoke<void>("write_terminal", { id, data });
export const resizeTerminal = (id: string, cols: number, rows: number) =>
  invoke<void>("resize_terminal", { id, cols, rows });
/** The folder a terminal's shell is in now; null when it can't be told. */
export const terminalCwd = (id: string) => invoke<string | null>("terminal_cwd", { id });
export const closeTerminal = (id: string) => invoke<void>("close_terminal", { id });
