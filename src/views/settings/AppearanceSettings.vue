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
import { isMac, isWindows } from "../../platform";
import { DEFAULT_SETTINGS, settings, updateSettings } from "../../settings";
import { THEME_OPTIONS } from "../../theme";

const zoomKeys = isMac ? "⌘+ and ⌘−" : "Ctrl+Plus and Ctrl+Minus";

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
const PANE_DIM_MAX = 0.8;

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
          <div class="brand">Appearance</div>
          <p class="muted tiny">Theme, font, how see-through the window is, and how much unfocused panes dim.</p>
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
        <div class="settings-row">
          <div class="settings-row-copy">
            <h3>Dim unfocused panes</h3>
            <p class="muted tiny">How much the other panes in a split tab fade while one has focus. 0% turns dimming off.</p>
          </div>
          <div class="settings-control settings-slider">
            <label class="settings-slider-input">
              <span class="visually-hidden">Dim unfocused panes</span>
              <input
                type="range"
                min="0"
                :max="PANE_DIM_MAX"
                step="0.05"
                :value="settings.paneDim"
                @input="updateSettings({ paneDim: numberFrom($event) })"
              />
            </label>
            <span class="settings-slider-value">{{ settings.paneDim === 0 ? "Off" : `${Math.round(settings.paneDim * 100)}%` }}</span>
            <button
              class="ghost tiny"
              type="button"
              :disabled="settings.paneDim === DEFAULT_SETTINGS.paneDim"
              @click="updateSettings({ paneDim: DEFAULT_SETTINGS.paneDim })"
            >
              Reset
            </button>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>
