<script setup lang="ts">
import { onMounted, ref } from "vue";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import * as api from "../../api";
import { settings, updateSettings } from "../../settings";
import { THEME_OPTIONS } from "../../theme";
import { showToast } from "../../toast";

const SCROLLBACK_MIN = 500;
const SCROLLBACK_MAX = 100_000;

const settingsPath = ref("");

onMounted(async () => {
  settingsPath.value = await api.settingsFilePath().catch(() => "");
});

function onShell(event: Event) {
  updateSettings({ shell: (event.target as HTMLInputElement).value.trim() });
}

function onScrollback(event: Event) {
  const input = event.target as HTMLInputElement;
  const value = Math.round(Number(input.value));
  if (!Number.isFinite(value)) {
    input.value = String(settings.value.scrollback);
    return;
  }
  const clamped = Math.min(SCROLLBACK_MAX, Math.max(SCROLLBACK_MIN, value));
  input.value = String(clamped);
  updateSettings({ scrollback: clamped });
}

async function revealSettings() {
  try {
    await revealItemInDir(settingsPath.value);
  } catch (err) {
    showToast(api.errorMessage(err), "error");
  }
}
</script>

<template>
  <div class="settings-pane settings-form-page">
    <div class="settings-inner">
      <div class="settings-header">
        <div>
          <div class="brand">General</div>
          <p class="muted tiny">Theme, the shell new tabs start, and how much history they keep.</p>
        </div>
      </div>

      <section class="settings-card">
        <div class="settings-row">
          <div class="settings-row-copy">
            <h3>Theme</h3>
            <p class="muted tiny">System follows your light or dark appearance setting.</p>
          </div>
          <div class="segmented" role="group" aria-label="Theme">
            <button
              v-for="option in THEME_OPTIONS"
              :key="option.id"
              type="button"
              :class="{ active: settings.theme === option.id }"
              :aria-pressed="settings.theme === option.id"
              @click="updateSettings({ theme: option.id })"
            >
              {{ option.label }}
            </button>
          </div>
        </div>
        <div class="settings-row">
          <div class="settings-row-copy">
            <h3>Shell</h3>
            <p class="muted tiny">Leave blank to use your login shell. Applies to new tabs.</p>
          </div>
          <div class="settings-control settings-text">
            <input
              type="text"
              :value="settings.shell"
              placeholder="Login shell"
              autocapitalize="off"
              autocorrect="off"
              autocomplete="off"
              spellcheck="false"
              @change="onShell"
            />
          </div>
        </div>
        <div class="settings-row">
          <div class="settings-row-copy">
            <h3>Scrollback</h3>
            <p class="muted tiny">Lines each tab keeps above the screen.</p>
          </div>
          <div class="settings-fields">
            <label class="settings-field">
              <span class="visually-hidden">Scrollback lines</span>
              <input
                type="number"
                :min="SCROLLBACK_MIN"
                :max="SCROLLBACK_MAX"
                step="500"
                :value="settings.scrollback"
                @change="onScrollback"
              />
            </label>
          </div>
        </div>
        <div class="settings-row">
          <div class="settings-row-copy">
            <h3>Settings file</h3>
            <p class="muted tiny settings-path" :title="settingsPath">{{ settingsPath || "…" }}</p>
          </div>
          <div class="settings-control">
            <button class="ghost" type="button" :disabled="!settingsPath" @click="revealSettings">Reveal</button>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>
