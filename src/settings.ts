import { ref, watch } from "vue";
import * as api from "./api";
import { clampFontSize, DEFAULT_FONT_SIZE } from "./fonts";
import { isMac } from "./platform";
import { sanitizeTheme, themePreference } from "./theme";
import { showToast } from "./toast";
import type { Settings } from "./types";

export const DEFAULT_SETTINGS: Settings = {
  theme: "system",
  fontFamily: "jetbrains",
  fontSize: DEFAULT_FONT_SIZE,
  lineHeight: 1.2,
  cursorStyle: "bar",
  cursorBlink: true,
  shell: "",
  startDirectory: "~",
  scrollback: 10_000,
  opacity: 0.8,
  blur: 8,
  paneDim: 0.4,
  dispatchPrompt: true,
  typeBelowPrompt: true,
  keepRunningInBackground: true,
  toggleHotkey: isMac ? "Cmd+Backquote" : "Ctrl+Backquote",
  autoCheckUpdates: true,
};

export const settings = ref<Settings>({ ...DEFAULT_SETTINGS, theme: themePreference.value });
export const settingsLoaded = ref(false);

watch(
  () => settings.value.theme,
  (theme) => {
    themePreference.value = sanitizeTheme(theme);
  },
);

/* The native blur lives on the window, so push it whenever the setting changes. */
watch(
  () => [settingsLoaded.value, settings.value.blur] as const,
  ([loaded, blur]) => {
    if (loaded) void api.setWindowBlur(Math.round(blur)).catch(() => undefined);
  },
  { immediate: true },
);

export async function loadSettings() {
  try {
    settings.value = { ...DEFAULT_SETTINGS, ...(await api.getSettings()) };
  } catch (err) {
    showToast(`Could not load settings: ${api.errorMessage(err)}`, "error");
  } finally {
    settingsLoaded.value = true;
  }
}

let saveTimer: number | undefined;

/**
 * Replaces every setting at once, as the JSON tab's Save and Import do. Missing
 * keys fall back to their defaults, and the toggle hotkey is re-registered if it changed.
 */
export async function replaceSettings(next: unknown): Promise<Settings> {
  if (!next || typeof next !== "object" || Array.isArray(next)) {
    throw new Error("Settings must be a JSON object.");
  }
  const merged: Settings = { ...DEFAULT_SETTINGS, ...(next as Partial<Settings>) };
  merged.theme = sanitizeTheme(merged.theme);
  if (merged.toggleHotkey !== settings.value.toggleHotkey) {
    await api.setToggleHotkey(merged.toggleHotkey);
  }
  window.clearTimeout(saveTimer);
  const saved = await api.saveSettings(merged);
  settings.value = { ...DEFAULT_SETTINGS, ...saved };
  return settings.value;
}

/** Applies a change at once and writes settings.json shortly after. */
export function updateSettings(patch: Partial<Settings>) {
  settings.value = { ...settings.value, ...patch };
  window.clearTimeout(saveTimer);
  saveTimer = window.setTimeout(() => {
    api.saveSettings(settings.value).catch((err) => {
      showToast(`Could not save settings: ${api.errorMessage(err)}`, "error");
    });
  }, 250);
}

export function zoom(step: number | "reset") {
  const next =
    step === "reset" ? DEFAULT_FONT_SIZE : clampFontSize(settings.value.fontSize + step, DEFAULT_FONT_SIZE);
  updateSettings({ fontSize: next });
}
