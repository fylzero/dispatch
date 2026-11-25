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
  /** How much unfocused panes in a split tab are dimmed, 0 (off) to 0.8. */
  paneDim: number;
  /** zsh and bash show Dispatch's prompt: Node version, folder, git branch, changed files. New tabs only. */
  dispatchPrompt: boolean;
  /** With the Dispatch prompt, commands are typed on the line below its chips instead of after them. New tabs only. */
  typeBelowPrompt: boolean;
  /** Close and Command+Q hide Dispatch; its tabs keep running. */
  keepRunningInBackground: boolean;
  /** Global shortcut that shows or hides Dispatch, like "Cmd+Backquote". Empty is off. */
  toggleHotkey: string;
  /** Look for a new release at launch and every few hours, and say when one is ready. */
  autoCheckUpdates: boolean;
}

