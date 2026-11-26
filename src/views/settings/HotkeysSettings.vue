<script setup lang="ts">
import * as api from "../../api";
import HotkeyRecorder from "../../components/HotkeyRecorder.vue";
import { formatAccelerator } from "../../hotkeys";
import { settings, updateSettings } from "../../settings";
import { showToast } from "../../toast";

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
          <div class="brand">Hotkeys</div>
          <p class="muted tiny">Global keyboard shortcuts. Click a shortcut to record a new one; Esc cancels, Delete turns it off.</p>
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
      </section>
    </div>
  </div>
</template>
