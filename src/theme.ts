import { computed, ref, watch } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { ITheme } from "@xterm/xterm";
import type { ResolvedTheme, ThemePreference } from "./types";

/** The terminal's colors in each theme, shared by the panes and the font preview in Settings. */
export const TERMINAL_THEMES: Record<ResolvedTheme, ITheme> = {
  dark: {
    background: "#08090c",
    foreground: "#e8edf5",
    cursor: "#3dd6b4",
    cursorAccent: "#08090c",
    selectionBackground: "#2a3344",
    black: "#1c2129",
    red: "#f07178",
    green: "#3dd68c",
    yellow: "#e6c07b",
    blue: "#7aa2f7",
    magenta: "#c678dd",
    cyan: "#56b6c2",
    white: "#d7deea",
    brightBlack: "#66707f",
    brightRed: "#ff8b93",
    brightGreen: "#5ee6a3",
    brightYellow: "#f0d090",
    brightBlue: "#9ab8ff",
    brightMagenta: "#d899ea",
    brightCyan: "#7ccbd5",
    brightWhite: "#ffffff",
  },
  light: {
    background: "#ffffff",
    foreground: "#1c2330",
    cursor: "#118777",
    cursorAccent: "#ffffff",
    selectionBackground: "#cfe0f7",
    black: "#1c2330",
    red: "#c9303a",
    green: "#15803d",
    yellow: "#a16207",
    blue: "#2c5fcf",
    magenta: "#9333ea",
    cyan: "#0e7490",
    white: "#6e7781",
    brightBlack: "#5f6b7d",
    brightRed: "#d1434b",
    brightGreen: "#178a52",
    brightYellow: "#b07a12",
    brightBlue: "#2f6fdb",
    brightMagenta: "#a855f7",
    brightCyan: "#0891b2",
    brightWhite: "#8c959f",
  },
};

export const THEME_OPTIONS: { id: ThemePreference; label: string }[] = [
  { id: "system", label: "System" },
  { id: "light", label: "Light" },
  { id: "dark", label: "Dark" },
];

/**
 * CodeMirror's own panels, search matches and selection, in the app's colors.
 * Spread into each editor's theme so these win over CodeMirror's defaults.
 */
export const editorChrome = {
  ".cm-selectionBackground, &.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground":
    { backgroundColor: "var(--selection)" },
  ".cm-panels": { backgroundColor: "var(--bg-raised)", color: "var(--text)" },
  ".cm-panels-top": { borderBottom: "1px solid var(--border)" },
  ".cm-panels-bottom": { borderTop: "1px solid var(--border)" },
  ".cm-textfield": {
    backgroundColor: "var(--bg-input)",
    border: "1px solid var(--border)",
    color: "var(--text)",
  },
  ".cm-button": {
    backgroundImage: "none",
    backgroundColor: "var(--bg-raised)",
    border: "1px solid var(--border)",
    color: "var(--text)",
  },
  ".cm-searchMatch": { backgroundColor: "var(--search-match)" },
  ".cm-searchMatch-selected": { backgroundColor: "var(--search-match-selected)" },
};

/** The last saved theme, so the window opens in it before the settings load. */
const STORAGE_KEY = "theme";

export function sanitizeTheme(value: unknown): ThemePreference {
  return value === "light" || value === "dark" ? value : "system";
}

function readStoredTheme() {
  try {
    return sanitizeTheme(localStorage.getItem(STORAGE_KEY));
  } catch {
    return "system";
  }
}

const systemQuery = window.matchMedia("(prefers-color-scheme: light)");
const systemTheme = ref<ResolvedTheme>(systemQuery.matches ? "light" : "dark");
systemQuery.addEventListener("change", (event) => {
  systemTheme.value = event.matches ? "light" : "dark";
});

export const themePreference = ref<ThemePreference>(readStoredTheme());

export const resolvedTheme = computed<ResolvedTheme>(() =>
  themePreference.value === "system" ? systemTheme.value : themePreference.value,
);

watch(
  resolvedTheme,
  (theme) => {
    document.documentElement.dataset.theme = theme;
  },
  { immediate: true },
);

/**
 * The native window follows the choice too, so the title bar matches. With
 * System it goes back to following the OS, which also keeps the
 * prefers-color-scheme query reporting the OS setting.
 */
async function applyWindowTheme(preference: ThemePreference) {
  try {
    await getCurrentWindow().setTheme(preference === "system" ? null : preference);
  } catch {
    /* not every platform can theme the native window */
  }
}

watch(
  themePreference,
  (preference) => {
    try {
      localStorage.setItem(STORAGE_KEY, preference);
    } catch {
      /* the cached theme only avoids a flash on launch */
    }
    void applyWindowTheme(preference);
  },
  { immediate: true },
);
