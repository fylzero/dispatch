<script setup lang="ts">
import { SETTINGS_SECTIONS, type SettingsSection, openSettings, settingsSection } from "../tabs";
import AppearanceSettings from "./settings/AppearanceSettings.vue";
import GeneralSettings from "./settings/GeneralSettings.vue";
import JsonSettings from "./settings/JsonSettings.vue";
import TerminalSettings from "./settings/TerminalSettings.vue";

const sections: { id: SettingsSection; label: string }[] = [
  { id: "general", label: "General" },
  { id: "appearance", label: "Appearance" },
  { id: "terminal", label: "Terminal" },
  { id: "json", label: "JSON" },
];

const isSection = (value: string) => (SETTINGS_SECTIONS as readonly string[]).includes(value);
</script>

<template>
  <div class="settings-layout">
    <nav class="settings-nav" aria-label="Settings">
      <button
        v-for="item in sections"
        :key="item.id"
        class="settings-nav-item"
        type="button"
        :class="{ active: settingsSection === item.id }"
        :aria-current="settingsSection === item.id ? 'page' : undefined"
        @click="openSettings(item.id)"
      >
        <svg v-if="item.id === 'general'" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M4 6h16" />
          <path d="M4 12h10" />
          <path d="M4 18h13" />
        </svg>
        <svg v-else-if="item.id === 'appearance'" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M4 19 10 5l6 14" />
          <path d="M6.5 14h7" />
          <path d="M17 19v-6" />
          <path d="M20 19v-6" />
        </svg>
        <svg v-else-if="item.id === 'json'" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" />
          <path d="M14 2v4a2 2 0 0 0 2 2h4" />
        </svg>
        <svg v-else viewBox="0 0 24 24" aria-hidden="true">
          <rect x="3.5" y="4.5" width="17" height="15" rx="2" />
          <path d="m7.5 9.5 3 2.5-3 2.5" />
          <path d="M12.5 15h4" />
        </svg>
        {{ item.label }}
      </button>
    </nav>
    <div class="settings-content" :class="{ fill: settingsSection === 'json' }">
      <AppearanceSettings v-if="settingsSection === 'appearance'" />
      <JsonSettings v-else-if="settingsSection === 'json'" />
      <TerminalSettings v-else-if="settingsSection === 'terminal'" />
      <GeneralSettings v-else-if="isSection(settingsSection)" />
    </div>
  </div>
</template>
