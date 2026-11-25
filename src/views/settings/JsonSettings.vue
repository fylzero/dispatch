<script setup lang="ts">
import { json } from "@codemirror/lang-json";
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { EditorView } from "@codemirror/view";
import { tags as t } from "@lezer/highlight";
import { basicSetup } from "codemirror";
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { confirm, open as openFile, save as saveFile } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import * as api from "../../api";
import { replaceSettings, settings } from "../../settings";
import { editorChrome } from "../../theme";
import { showToast } from "../../toast";
import type { Settings } from "../../types";

const editorHost = ref<HTMLDivElement | null>(null);
const draft = ref("");
const baseline = ref("");
const message = ref("");
const saving = ref(false);
const copied = ref(false);
const exporting = ref(false);
const exported = ref(false);
const importing = ref(false);
const settingsPath = ref("");
let view: EditorView | null = null;

const jsonDirty = computed(() => draft.value !== baseline.value);

const editorTheme = EditorView.theme(
  {
    "&": {
      height: "100%",
      color: "var(--text)",
      backgroundColor: "var(--bg-input)",
      fontFamily: "var(--font-code)",
      fontSize: "12px",
    },
    ...editorChrome,
    "&.cm-focused": { outline: "none" },
    ".cm-scroller": { fontFamily: "var(--font-code)", lineHeight: "1.5" },
    ".cm-content": { caretColor: "var(--text)", padding: "0.45rem 0.8rem" },
    ".cm-gutters": {
      backgroundColor: "var(--bg-input)",
      color: "var(--muted)",
      borderRight: "1px solid var(--border)",
    },
    ".cm-activeLine": { backgroundColor: "rgb(var(--tint) / 0.03)" },
    ".cm-activeLineGutter": { backgroundColor: "transparent" },
    ".cm-cursor": { borderLeftColor: "var(--text)" },
  },
  { dark: true },
);

const editorHighlight = HighlightStyle.define([
  { tag: t.propertyName, color: "var(--syntax-type)" },
  { tag: t.string, color: "var(--syntax-string)" },
  { tag: t.number, color: "var(--syntax-number)" },
  { tag: t.bool, color: "var(--syntax-constant)" },
  { tag: t.null, color: "var(--muted)" },
  { tag: t.punctuation, color: "var(--muted)" },
  { tag: t.squareBracket, color: "var(--syntax-bracket)" },
  { tag: t.brace, color: "var(--syntax-bracket)" },
  { tag: t.invalid, color: "var(--bad)" },
]);

const displayPath = computed(() => {
  const home = settingsPath.value.match(/^\/(?:Users|home)\/[^/]+/);
  return home ? settingsPath.value.replace(home[0], "~") : settingsPath.value;
});

const pretty = (data: Settings) => JSON.stringify(data, null, 2);

function setDraft(value: string) {
  draft.value = value;
  if (view && view.state.doc.toString() !== value) {
    view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: value } });
  }
}

/** Shows the current settings, making them the new baseline. */
function loadCurrent() {
  const value = pretty(settings.value);
  setDraft(value);
  baseline.value = value;
}

/* Follow changes made on the other tabs, unless there are unsaved edits here. */
watch(
  settings,
  () => {
    if (!jsonDirty.value) loadCurrent();
  },
  { deep: true },
);

function errorText(err: unknown) {
  return err instanceof SyntaxError ? "Settings JSON is not valid." : api.errorMessage(err);
}

onMounted(async () => {
  loadCurrent();
  settingsPath.value = await api.settingsFilePath().catch(() => "");
  await nextTick();
  if (!editorHost.value) return;
  view = new EditorView({
    doc: draft.value,
    parent: editorHost.value,
    extensions: [
      basicSetup,
      json(),
      editorTheme,
      syntaxHighlighting(editorHighlight),
      EditorView.updateListener.of((update) => {
        if (update.docChanged) draft.value = update.state.doc.toString();
      }),
    ],
  });
});

onUnmounted(() => {
  view?.destroy();
  view = null;
});

async function copy() {
  try {
    await navigator.clipboard.writeText(draft.value);
    copied.value = true;
    window.setTimeout(() => (copied.value = false), 1600);
  } catch (err) {
    message.value = errorText(err);
  }
}

async function revealFile() {
  message.value = "";
  try {
    await revealItemInDir(settingsPath.value);
  } catch {
    /* The file is only written after the first change, so write it and try again. */
    try {
      await api.saveSettings(settings.value);
      await revealItemInDir(settingsPath.value);
    } catch (err) {
      message.value = errorText(err);
    }
  }
}

async function exportSettings() {
  message.value = "";
  let contents: string;
  try {
    contents = pretty(JSON.parse(draft.value) as Settings);
  } catch (err) {
    message.value = errorText(err);
    return;
  }
  const path = await saveFile({
    title: "Export settings",
    defaultPath: "dispatch-settings.json",
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  if (!path) return;
  exporting.value = true;
  try {
    await api.writeTextFile(path, `${contents}\n`);
    exported.value = true;
    window.setTimeout(() => (exported.value = false), 1600);
  } catch (err) {
    message.value = errorText(err);
  } finally {
    exporting.value = false;
  }
}

async function importSettings() {
  message.value = "";
  const path = await openFile({
    title: "Import settings",
    multiple: false,
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  if (!path || Array.isArray(path)) return;
  importing.value = true;
  try {
    const parsed: unknown = JSON.parse(await api.readTextFile(path));
    const confirmed = await confirm("Replace all settings with this file?", {
      title: "Import settings",
      kind: "warning",
      okLabel: "Import",
      cancelLabel: "Cancel",
    });
    if (!confirmed) return;
    await replaceSettings(parsed);
    loadCurrent();
    showToast("Settings have been imported.");
  } catch (err) {
    message.value = errorText(err);
  } finally {
    importing.value = false;
  }
}

function discard() {
  message.value = "";
  loadCurrent();
}

async function save() {
  message.value = "";
  let parsed: unknown;
  try {
    parsed = JSON.parse(draft.value);
  } catch (err) {
    message.value = errorText(err);
    return;
  }
  saving.value = true;
  try {
    await replaceSettings(parsed);
    loadCurrent();
    showToast("Settings have been saved.");
  } catch (err) {
    message.value = errorText(err);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <div class="settings-pane settings-json-pane">
    <div class="settings-inner">
      <div class="settings-header">
        <div>
          <div class="brand">JSON</div>
          <p class="muted tiny settings-path" :title="settingsPath">
            {{ displayPath || "Edit the saved settings file here, or import a backup." }}
          </p>
        </div>
        <div class="settings-header-actions">
          <button class="ghost" type="button" :disabled="!settingsPath" @click="revealFile">Reveal</button>
          <button class="ghost" type="button" :disabled="importing" @click="importSettings">
            {{ importing ? "Importing…" : "Import" }}
          </button>
          <button class="ghost" type="button" :disabled="exporting || !draft.trim()" @click="exportSettings">
            <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path
                d="M9 8.25H7.5a2.25 2.25 0 0 0-2.25 2.25v9a2.25 2.25 0 0 0 2.25 2.25h9a2.25 2.25 0 0 0 2.25-2.25v-9A2.25 2.25 0 0 0 16.5 8.25H15M9 12l3 3m0 0 3-3m-3 3V2.25"
              />
            </svg>
            {{ exporting ? "Exporting…" : exported ? "Exported" : "Export" }}
          </button>
          <button v-if="jsonDirty" class="ghost" type="button" :disabled="saving" @click="discard">Discard</button>
          <button class="primary" type="button" :disabled="saving || !jsonDirty || !draft.trim()" @click="save">
            {{ saving ? "Saving…" : "Save" }}
          </button>
        </div>
      </div>
      <div class="settings-editor">
        <button class="ghost tiny settings-copy" type="button" @click="copy">{{ copied ? "Copied" : "Copy" }}</button>
        <div ref="editorHost" class="settings-json" />
      </div>
      <p v-if="message" class="settings-error">{{ message }}</p>
    </div>
  </div>
</template>
