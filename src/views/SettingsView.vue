<script setup lang="ts">
import { SETTINGS_SECTIONS, type SettingsSection, openSettings, settingsSection } from "../tabs";
import GeneralSettings from "./settings/GeneralSettings.vue";
import TerminalSettings from "./settings/TerminalSettings.vue";

const sections: { id: SettingsSection; label: string }[] = [
  { id: "general", label: "General" },
  { id: "terminal", label: "Terminal" },
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
        <svg v-else viewBox="0 0 24 24" aria-hidden="true">
          <rect x="3.5" y="4.5" width="17" height="15" rx="2" />
          <path d="m7.5 9.5 3 2.5-3 2.5" />
          <path d="M12.5 15h4" />
        </svg>
        {{ item.label }}
      </button>
    </nav>
    <div class="settings-content">
      <TerminalSettings v-if="settingsSection === 'terminal'" />
      <GeneralSettings v-else-if="isSection(settingsSection)" />
    </div>
  </div>
</template>
