<script setup lang="ts">
import { isMac } from "../../platform";
import { settings, updateSettings } from "../../settings";
import type { CursorStyle, SuggestionAcceptKey } from "../../types";

const CURSOR_OPTIONS: { id: CursorStyle; label: string }[] = [
  { id: "block", label: "Block" },
  { id: "bar", label: "Bar" },
  { id: "underline", label: "Underline" },
];

const ACCEPT_OPTIONS: { id: SuggestionAcceptKey; label: string }[] = [
  { id: "tab", label: "Tab" },
  { id: "right", label: "Right arrow" },
  { id: "both", label: "Both" },
];

/* Command+Right sends Ctrl+E (end of line), so it fills in a suggestion like End. */
const acceptNote = isMac
  ? "Right arrow also covers End and Command+Right, and Option+Right takes one word."
  : "Right arrow also covers End, and Ctrl+Right takes one word.";

</script>

<template>
  <div class="settings-pane settings-form-page">
    <div class="settings-inner">
      <div class="settings-header">
        <div>
          <div class="brand">Terminal</div>
          <p class="muted tiny">Cursor, prompt and suggestions.</p>
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
        <div class="settings-row">
          <div class="settings-row-copy">
            <h3>Suggestions</h3>
            <p class="muted tiny">
              As you type, shows in grey the file or folder you are typing, or a command when nothing matches. Enter runs only
              what you typed. Works in zsh and bash while the Dispatch prompt is on, and in PowerShell and cmd, where
              turning it on or off applies to new tabs.
            </p>
          </div>
          <div class="settings-control">
            <button
              class="switch"
              :class="{ on: settings.suggestions }"
              type="button"
              role="switch"
              :aria-checked="settings.suggestions"
              aria-label="Suggestions"
              @click="updateSettings({ suggestions: !settings.suggestions })"
            >
              <span class="switch-track" aria-hidden="true"><span class="switch-knob" /></span>
            </button>
          </div>
        </div>
        <div class="settings-row" :class="{ disabled: !settings.suggestions }">
          <div class="settings-row-copy">
            <h3>Accept suggestion with</h3>
            <p class="muted tiny">
              With nothing suggested, Tab completes as usual. {{ acceptNote }} Esc hides it.
            </p>
          </div>
          <div class="segmented" role="group" aria-label="Accept suggestion with">
            <button
              v-for="option in ACCEPT_OPTIONS"
              :key="option.id"
              type="button"
              :class="{ active: settings.suggestionAcceptKey === option.id }"
              :aria-pressed="settings.suggestionAcceptKey === option.id"
              :disabled="!settings.suggestions"
              @click="updateSettings({ suggestionAcceptKey: option.id })"
            >
              {{ option.label }}
            </button>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>

