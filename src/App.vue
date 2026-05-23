<script setup lang="ts">
import { onMounted } from "vue";
import { checkForAppUpdates } from './updater';
import { useRuntimeStore } from "./stores/runtime";
import {useVolumesStore} from "./stores/volumes";
import {useNetworksStore} from "./stores/networks";
import {useSettingsStore} from "./stores/settings.ts";

import { useWorkspacesStore } from "./stores/workspaces";

const runtimeStore = useRuntimeStore();
const volumesStore = useVolumesStore();
const networksStore = useNetworksStore();
const settingsStore = useSettingsStore();
const workspacesStore = useWorkspacesStore();

onMounted(async () => {
  // Vérification automatique au démarrage (silencieuse si pas de mise à jour)
  await checkForAppUpdates();
  await settingsStore.loadPersistedSettings();

  // Charger les espaces de travail et les images au démarrage
  await workspacesStore.fetchWorkspaces();

  // Charger les activités récentes
  await runtimeStore.loadRecentRuns();
  await volumesStore.initDockerEventsListener();
  await networksStore.initDockerEventsListener();
  
});
</script>

<template>
  <RouterView />
</template>

<style>
:root {
  --bg-main: #0d1117;
  --bg-panel: #161b22;
  --border-color: #30363d;
  --text-primary: #f0f6fc;
  --text-secondary: #8b949e;
  --accent-blue: #58a6ff;
}

* {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
  font-family: "Inter", "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
}

body {
  background-color: var(--bg-main);
  color: var(--text-primary);
  overflow: hidden;
}

/* Scrollbar styling */
::-webkit-scrollbar {
  width: 8px;
}
::-webkit-scrollbar-track {
  background: #0d1117;
}
::-webkit-scrollbar-thumb {
  background: #30363d;
  border-radius: 4px;
}
::-webkit-scrollbar-thumb:hover {
  background: #484f58;
}
</style>