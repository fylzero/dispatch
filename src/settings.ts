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
  lineHeight: 1.3,
  cursorStyle: "block",
  cursorBlink: true,
  shell: "",
  startDirectory: "~",
  scrollback: 10_000,
  opacity: 0.8,
  blur: 12,
  dispatchPrompt: true,
  keepRunningInBackground: true,
  toggleHotkey: isMac ? "Cmd+Backquote" : "Ctrl+Backquote",
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
