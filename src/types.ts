export type ThemePreference = "system" | "light" | "dark";
export type ResolvedTheme = "light" | "dark";
export type CursorStyle = "block" | "bar" | "underline";

/** Mirrors `Settings` in src-tauri/src/settings.rs. */
export interface Settings {
  theme: ThemePreference;
  fontFamily: string;
  fontSize: number;
  lineHeight: number;
  cursorStyle: CursorStyle;
  cursorBlink: boolean;
  /** Blank means the login shell from $SHELL. */
  shell: string;
  /** Folder new tabs start in; `~` is home. Blank or missing folders fall back to home. */
  startDirectory: string;
  scrollback: number;
  /** Terminal background opacity, 0.3–1. */
  opacity: number;
  /** Blur radius behind the see-through background; 0 is off. */
  blur: number;
  /** zsh and bash show Dispatch's prompt: Node version, folder, git branch, changed files. New tabs only. */
  dispatchPrompt: boolean;
  /** Close and Command+Q hide Dispatch; its tabs keep running. */
  keepRunningInBackground: boolean;
  /** Global shortcut that shows or hides Dispatch, like "Cmd+Backquote". Empty is off. */
  toggleHotkey: string;
}

