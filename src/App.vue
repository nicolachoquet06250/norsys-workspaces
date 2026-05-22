<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { checkForAppUpdates } from './updater';
import Sidebar from './components/layout/Sidebar.vue';
import { useRuntimeStore } from "./stores/runtime";
import {useVolumesStore} from "./stores/volumes";
import {useNetworksStore} from "./stores/networks";
import {useSettingsStore} from "./stores/settings.ts";

import { useWorkspacesStore } from "./stores/workspaces";

const isSidebarCollapsed = ref(window.innerWidth <= 768);
const isMobile = ref(window.innerWidth <= 768);
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
  
  window.addEventListener('resize', () => {
    isMobile.value = window.innerWidth <= 768;
    if (window.innerWidth > 768) {
      isSidebarCollapsed.value = false;
    } else if (!isSidebarCollapsed.value && isMobile.value) {
      // Keep open if it was already open during resize to mobile? 
      // Actually usually we want it collapsed by default on mobile resize
      isSidebarCollapsed.value = true;
    }
  });
});

function toggleSidebar() {
  isSidebarCollapsed.value = !isSidebarCollapsed.value;
}

function closeSidebar() {
  if (window.innerWidth <= 768 && !isSidebarCollapsed.value) {
    isSidebarCollapsed.value = true;
  }
}
</script>

<template>
  <div class="app-layout" :class="{ 'sidebar-collapsed': isSidebarCollapsed }">
    <div 
      v-if="!isSidebarCollapsed && isMobile" 
      class="sidebar-overlay" 
      @click="closeSidebar"
    ></div>
    <Sidebar :is-collapsed="isSidebarCollapsed" @toggle="toggleSidebar" />
    <main class="app-main" @click="closeSidebar">
      <RouterView />
    </main>
  </div>
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

.app-layout {
  display: flex;
  height: 100vh;
  width: 100vw;
  position: relative;
}

.sidebar-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: rgba(0, 0, 0, 0.5);
  z-index: 999;
  backdrop-filter: blur(2px);
}

.app-main {
  flex-grow: 1;
  overflow-y: auto;
  background-color: #0d1117;
  transition: margin-left 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

@media (max-width: 768px) {
  .app-main {
    margin-left: 70px;
  }
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