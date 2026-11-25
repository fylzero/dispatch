<script setup lang="ts">
import * as api from "../../api";
import HotkeyRecorder from "../../components/HotkeyRecorder.vue";
import { formatAccelerator } from "../../hotkeys";
import { settings, updateSettings } from "../../settings";
import { showToast } from "../../toast";

const SCROLLBACK_MIN = 500;
const SCROLLBACK_MAX = 100_000;

function onShell(event: Event) {
  updateSettings({ shell: (event.target as HTMLInputElement).value.trim() });
}

function onStartDirectory(event: Event) {
  updateSettings({ startDirectory: (event.target as HTMLInputElement).value.trim() || "~" });
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

async function onToggleHotkey(accelerator: string) {
  try {
    await api.setToggleHotkey(accelerator);
    updateSettings({ toggleHotkey: accelerator });
    showToast(accelerator ? `${formatAccelerator(accelerator)} now shows and hides Dispatch.` : "Toggle hotkey turned off.");
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
          <p class="muted tiny">The toggle hotkey, updates, the shell and folder new tabs start in, and how much history they keep.</p>
        </div>
      </div>

      <section class="settings-card">
        <div class="settings-row">
          <div class="settings-row-copy">
            <h3>Toggle hotkey</h3>
            <p class="muted tiny">Shows or hides Dispatch from any app. Click to record a new one; Delete turns it off.</p>
          </div>
          <div class="settings-control">
            <HotkeyRecorder
              label="Toggle Dispatch"
              :value="settings.toggleHotkey"
              @change="onToggleHotkey"
              @invalid="(message) => showToast(message, 'error')"
            />
          </div>
        </div>
        <div class="settings-row">
          <div class="settings-row-copy">
            <h3>Keep running in the background</h3>
            <p class="muted tiny">
              Closing the window or quitting hides Dispatch and keeps your tabs running, so the hotkey brings them
              straight back. Quit from the menu bar icon to really quit.
            </p>
          </div>
          <div class="settings-control">
            <button
              class="switch"
              :class="{ on: settings.keepRunningInBackground }"
              type="button"
              role="switch"
              :aria-checked="settings.keepRunningInBackground"
              aria-label="Keep running in the background"
              @click="updateSettings({ keepRunningInBackground: !settings.keepRunningInBackground })"
            >
              <span class="switch-track" aria-hidden="true"><span class="switch-knob" /></span>
            </button>
          </div>
        </div>
        <div class="settings-row">
          <div class="settings-row-copy">
            <h3>Check for updates automatically</h3>
            <p class="muted tiny">
              Looks for a new version when Dispatch starts and every few hours. You get a notification when one is
              ready, and Dispatch asks before installing it.
            </p>
          </div>
          <div class="settings-control">
            <button
              class="switch"
              :class="{ on: settings.autoCheckUpdates }"
              type="button"
              role="switch"
              :aria-checked="settings.autoCheckUpdates"
              aria-label="Check for updates automatically"
              @click="updateSettings({ autoCheckUpdates: !settings.autoCheckUpdates })"
            >
              <span class="switch-track" aria-hidden="true"><span class="switch-knob" /></span>
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
            <h3>Starting directory</h3>
            <p class="muted tiny">Folder new tabs open in. ~ is your home folder, which is also used if this one is missing.</p>
          </div>
          <div class="settings-control settings-text">
            <input
              type="text"
              :value="settings.startDirectory"
              placeholder="~"
              autocapitalize="off"
              autocorrect="off"
              autocomplete="off"
              spellcheck="false"
              @change="onStartDirectory"
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
      </section>
    </div>
  </div>
</template>
