<script setup lang="ts">
import { computed, ref, watch } from "vue";
import {
  CUSTOM_FONT_ID,
  DEFAULT_FONT_SIZE,
  FONT_OPTIONS,
  FONT_SIZE_MAX,
  FONT_SIZE_MIN,
  formatFontSize,
  isPresetFont,
} from "../../fonts";
import { DEFAULT_SETTINGS, settings, updateSettings } from "../../settings";
import { isMac, isWindows } from "../../platform";
import type { CursorStyle } from "../../types";

const zoomKeys = isMac ? "⌘+ and ⌘−" : "Ctrl+Plus and Ctrl+Minus";

const CURSOR_OPTIONS: { id: CursorStyle; label: string }[] = [
  { id: "block", label: "Block" },
  { id: "bar", label: "Bar" },
  { id: "underline", label: "Underline" },
];

const usingCustom = ref(!isPresetFont(settings.value.fontFamily));
const customFont = ref(usingCustom.value ? settings.value.fontFamily : "");

watch(
  () => settings.value.fontFamily,
  (value) => {
    usingCustom.value = !isPresetFont(value);
    if (usingCustom.value) customFont.value = value;
  },
);

function onFont(event: Event) {
  const value = (event.target as HTMLSelectElement).value;
  if (value === CUSTOM_FONT_ID) {
    usingCustom.value = true;
    if (customFont.value.trim()) updateSettings({ fontFamily: customFont.value.trim() });
    return;
  }
  usingCustom.value = false;
  updateSettings({ fontFamily: value });
}

function onCustomFont(event: Event) {
  const value = (event.target as HTMLInputElement).value.trim();
  customFont.value = value;
  if (value) updateSettings({ fontFamily: value });
}

const OPACITY_MIN = 0.3;
const BLUR_MAX = 40;

/* macOS takes a blur radius; Windows only has its on/off Acrylic blur; Linux has none. */
const blurSupported = isMac || isWindows;
const blurHint = isMac
  ? "Softens what shows through the background. Needs opacity below 100%."
  : isWindows
    ? "Turns on the Windows Acrylic blur behind the background. Windows has no strength setting, so any amount is the same."
    : "Linux has no standard way to blur behind a window.";
const blurLabel = computed(() => {
  if (!blurSupported || settings.value.blur === 0) return "Off";
  return isWindows ? "On" : `${Math.round(settings.value.blur)}px`;
});

const numberFrom = (event: Event) => Number((event.target as HTMLInputElement).value);
</script>

<template>
  <div class="settings-pane settings-form-page">
    <div class="settings-inner">
      <div class="settings-header">
        <div>
          <div class="brand">Terminal</div>
          <p class="muted tiny">Font, cursor, prompt, and how see-through the window is.</p>
        </div>
      </div>

      <section class="settings-card">
        <div class="settings-row" :class="{ 'settings-row-stacked': usingCustom }">
          <div class="settings-row-copy">
            <h3>Font</h3>
            <p class="muted tiny">Typeface for terminal text.</p>
          </div>
          <div class="settings-control settings-font">
            <label>
              <span class="visually-hidden">Font</span>
              <select :value="usingCustom ? CUSTOM_FONT_ID : settings.fontFamily" @change="onFont">
                <option v-for="option in FONT_OPTIONS" :key="option.id" :value="option.id">
                  {{ option.label }}
                </option>
                <option :value="CUSTOM_FONT_ID">Custom…</option>
              </select>
            </label>
            <label v-if="usingCustom">
              <span class="visually-hidden">Custom font</span>
              <input
                type="text"
                :value="customFont"
                placeholder="Font family name"
                autocapitalize="off"
                autocorrect="off"
                autocomplete="off"
                spellcheck="false"
                @change="onCustomFont"
              />
            </label>
          </div>
        </div>
        <div class="settings-row">
          <div class="settings-row-copy">
            <h3>Font size</h3>
            <p class="muted tiny">Also changes with {{ zoomKeys }}.</p>
          </div>
          <div class="settings-control settings-slider">
            <label class="settings-slider-input">
              <span class="visually-hidden">Font size</span>
              <input
                type="range"
                :min="FONT_SIZE_MIN"
                :max="FONT_SIZE_MAX"
                step="0.5"
                :value="settings.fontSize"
                @input="updateSettings({ fontSize: numberFrom($event) })"
              />
            </label>
            <span class="settings-slider-value">{{ formatFontSize(settings.fontSize) }}</span>
            <button
              class="ghost tiny"
              type="button"
              :disabled="settings.fontSize === DEFAULT_FONT_SIZE"
              @click="updateSettings({ fontSize: DEFAULT_FONT_SIZE })"
            >
              Reset
            </button>
          </div>
        </div>
        <div class="settings-row">
          <div class="settings-row-copy">
            <h3>Line height</h3>
            <p class="muted tiny">Space between lines, as a multiple of the font size.</p>
          </div>
          <div class="settings-control settings-slider">
            <label class="settings-slider-input">
              <span class="visually-hidden">Line height</span>
              <input
                type="range"
                min="1"
                max="2"
                step="0.05"
                :value="settings.lineHeight"
                @input="updateSettings({ lineHeight: numberFrom($event) })"
              />
            </label>
            <span class="settings-slider-value">{{ settings.lineHeight.toFixed(2) }}</span>
            <button
              class="ghost tiny"
              type="button"
              :disabled="settings.lineHeight === DEFAULT_SETTINGS.lineHeight"
              @click="updateSettings({ lineHeight: DEFAULT_SETTINGS.lineHeight })"
            >
              Reset
            </button>
          </div>
        </div>
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
            <h3>Window opacity</h3>
            <p class="muted tiny">How solid the terminal background is. Lower shows the desktop through it.</p>
          </div>
          <div class="settings-control settings-slider">
            <label class="settings-slider-input">
              <span class="visually-hidden">Window opacity</span>
              <input
                type="range"
                :min="OPACITY_MIN"
                max="1"
                step="0.05"
                :value="settings.opacity"
                @input="updateSettings({ opacity: numberFrom($event) })"
              />
            </label>
            <span class="settings-slider-value">{{ Math.round(settings.opacity * 100) }}%</span>
            <button
              class="ghost tiny"
              type="button"
              :disabled="settings.opacity === DEFAULT_SETTINGS.opacity"
              @click="updateSettings({ opacity: DEFAULT_SETTINGS.opacity })"
            >
              Reset
            </button>
          </div>
        </div>
        <div class="settings-row">
          <div class="settings-row-copy">
            <h3>Background blur</h3>
            <p class="muted tiny">{{ blurHint }}</p>
          </div>
          <div class="settings-control settings-slider">
            <label class="settings-slider-input">
              <span class="visually-hidden">Background blur</span>
              <input
                type="range"
                min="0"
                :max="BLUR_MAX"
                step="1"
                :value="settings.blur"
                :disabled="!blurSupported"
                @input="updateSettings({ blur: numberFrom($event) })"
              />
            </label>
            <span class="settings-slider-value">{{ blurLabel }}</span>
            <button
              class="ghost tiny"
              type="button"
              :disabled="!blurSupported || settings.blur === DEFAULT_SETTINGS.blur"
              @click="updateSettings({ blur: DEFAULT_SETTINGS.blur })"
            >
              Reset
            </button>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>
