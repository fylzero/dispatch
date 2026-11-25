<script setup lang="ts">
import { settings, updateSettings } from "../../settings";
import type { CursorStyle } from "../../types";

const CURSOR_OPTIONS: { id: CursorStyle; label: string }[] = [
  { id: "block", label: "Block" },
  { id: "bar", label: "Bar" },
  { id: "underline", label: "Underline" },
];
</script>

<template>
  <div class="settings-pane settings-form-page">
    <div class="settings-inner">
      <div class="settings-header">
        <div>
          <div class="brand">Terminal</div>
          <p class="muted tiny">Cursor and prompt.</p>
        </div>
      </div>

      <section class="settings-card">
        <div class="settings-row">
          <div class="settings-row-copy">
            <h3>Cursor</h3>
            <p class="muted tiny">Shape of the text cursor.</p>
          </div>
          <div class="segmented" role="group" aria-label="Cursor">
            <button
              v-for="option in CURSOR_OPTIONS"
              :key="option.id"
              type="button"
              :class="{ active: settings.cursorStyle === option.id }"
              :aria-pressed="settings.cursorStyle === option.id"
              @click="updateSettings({ cursorStyle: option.id })"
            >
              {{ option.label }}
            </button>
          </div>
        </div>
        <div class="settings-row">
          <div class="settings-row-copy">
            <h3>Blinking cursor</h3>
            <p class="muted tiny">Blink the cursor while a tab has focus.</p>
          </div>
          <div class="settings-control">
            <button
              class="switch"
              :class="{ on: settings.cursorBlink }"
              type="button"
              role="switch"
              :aria-checked="settings.cursorBlink"
              aria-label="Blinking cursor"
              @click="updateSettings({ cursorBlink: !settings.cursorBlink })"
            >
              <span class="switch-track" aria-hidden="true"><span class="switch-knob" /></span>
            </button>
          </div>
        </div>
        <div class="settings-row">
          <div class="settings-row-copy">
            <h3>Dispatch prompt</h3>
            <p class="muted tiny">
              Replaces the zsh or bash prompt with the Node version, folder, git branch, and changed files. Applies to
              new tabs.
            </p>
          </div>
          <div class="settings-control">
            <button
              class="switch"
              :class="{ on: settings.dispatchPrompt }"
              type="button"
              role="switch"
              :aria-checked="settings.dispatchPrompt"
              aria-label="Dispatch prompt"
              @click="updateSettings({ dispatchPrompt: !settings.dispatchPrompt })"
            >
              <span class="switch-track" aria-hidden="true"><span class="switch-knob" /></span>
            </button>
          </div>
        </div>
        <div class="settings-row" :class="{ disabled: !settings.dispatchPrompt }">
          <div class="settings-row-copy">
            <h3>Type below the prompt</h3>
            <p class="muted tiny">
              Puts the cursor on its own line under the Dispatch prompt. Turn it off to type right after the chips.
              Applies to new tabs.
            </p>
          </div>
          <div class="settings-control">
            <button
              class="switch"
              :class="{ on: settings.typeBelowPrompt }"
              type="button"
              role="switch"
              :aria-checked="settings.typeBelowPrompt"
              :disabled="!settings.dispatchPrompt"
              aria-label="Type below the prompt"
              @click="updateSettings({ typeBelowPrompt: !settings.typeBelowPrompt })"
            >
              <span class="switch-track" aria-hidden="true"><span class="switch-knob" /></span>
            </button>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>
