<script setup lang="ts">
import { useRoute, useRouter } from "vue-router";
import { onMounted, onUnmounted } from "vue";

import { useSettingsStore } from "../../stores/settings";

defineProps<{
  isCollapsed: boolean;
}>();

const emit = defineEmits<{
  (e: 'toggle'): void;
}>();

const route = useRoute();
const router = useRouter();
const settingsStore = useSettingsStore();

let interval: number | undefined;

onMounted(() => {
  // Rafraîchir l'état de Docker toutes les 30 secondes
  interval = window.setInterval(() => {
    void settingsStore.checkDockerConnection();
  }, 30000);
});

onUnmounted(() => {
  if (interval) clearInterval(interval);
});

const menuItems = [
  { name: "Accueil", icon: "🏠", path: "/" },
  { name: "Workspaces", icon: "📁", path: "/workspaces" },
  { name: "Services", icon: "🧩", path: "/services" },
  // TODO: Ne pas supprimer, sera utilisé plus tard
  // { name: "Environnements", icon: "🤖", path: "/environments" },
  { name: "Images", icon: "📦", path: "/images" },
  { name: "Volumes", icon: "💾", path: "/volumes" },
  { name: "Réseau", icon: "🌐", path: "/networks" },
  { name: "Logs", icon: "📋", path: "/logs" },
];

const secondaryItems = [
  { name: "Paramètres", icon: "⚙️", path: "/settings" },
  { name: "Extensions", icon: "🔌", path: "/extensions" },
];

function isActive(path: string) {
  if (path === "/" && route.path === "/") return true;
  return path !== "/" && route.path.startsWith(path);

}

function navigate(path: string) {
  router.push(path);
}
</script>

<template>
  <aside class="sidebar" :class="{ 'is-collapsed': isCollapsed }">
    <div class="logo" :class="{ 'clickable': isCollapsed }" @click="isCollapsed ? emit('toggle') : null">
      <div class="logo-icon">💠</div>
      <span class="logo-text">Dev Workspace Manager</span>
    </div>

    <nav class="menu-main">
      <ul>
        <li 
          v-for="item in menuItems" 
          :key="item.path"
          :class="{ active: isActive(item.path) }"
          @click="navigate(item.path)"
        >
          <span class="icon">{{ item.icon }}</span>
          <span class="label">{{ item.name }}</span>
        </li>
      </ul>
    </nav>

    <div class="spacer"></div>

    <nav class="menu-secondary">
      <ul>
        <li 
          v-for="item in secondaryItems" 
          :key="item.path"
          :class="{ active: isActive(item.path) }"
          @click="navigate(item.path)"
        >
          <span class="icon">{{ item.icon }}</span>
          <span class="label">{{ item.name }}</span>
        </li>
      </ul>
    </nav>

    <div class="user-profile">
      <div class="avatar">
        <img :src="`https://ui-avatars.com/api/?name=${encodeURIComponent(settingsStore.username)}&background=random`" :alt="settingsStore.username" />
      </div>
      <div class="user-info">
        <span class="name">{{ settingsStore.username }}</span>
        <span class="email">{{ settingsStore.email }}</span>
      </div>
    </div>

    <div class="docker-status">
      <div class="docker-info">
        <span class="icon">🐳</span>
        <span class="label">{{ settingsStore.isDockerConnected ? 'Docker connecté' : 'Docker déconnecté' }}</span>
      </div>
      <div class="status-dots">
        <span class="dot" :class="settingsStore.isDockerConnected ? 'green' : 'red'"></span>
        <span class="dot" :class="settingsStore.isDockerConnected ? 'green' : 'red'"></span>
      </div>
    </div>

    <div class="version">v1.2.0</div>
  </aside>
</template>

<style scoped>
.sidebar {
  width: 260px;
  background-color: #0d1117;
  color: #8b949e;
  display: flex;
  flex-direction: column;
  padding: 1.5rem 1rem;
  height: 100vh;
  border-right: 1px solid #21262d;
  transition: width 0.3s cubic-bezier(0.4, 0, 0.2, 1), padding 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  z-index: 1000;
  overflow: hidden;
}

@media (max-width: 768px) {
  .sidebar {
    position: fixed;
    left: 0;
    top: 0;
    box-shadow: 5px 0 15px rgba(0, 0, 0, 0.5);
    transition: width 0.3s cubic-bezier(0.4, 0, 0.2, 1), padding 0.3s cubic-bezier(0.4, 0, 0.2, 1), transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  }
  
  .sidebar.is-collapsed {
    position: fixed;
    box-shadow: none;
  }
}

.sidebar.is-collapsed {
  width: 70px;
  padding: 1.5rem 0.5rem;
}

.logo-text, 
.label, 
.user-info,
.docker-status, 
.version {
  transition: opacity 0.2s ease, visibility 0.2s, width 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  opacity: 1;
  visibility: visible;
  white-space: nowrap;
  user-select: none;
}

.sidebar:not(.is-collapsed) .logo-text,
.sidebar:not(.is-collapsed) .label,
.sidebar:not(.is-collapsed) .user-info,
.sidebar:not(.is-collapsed) .docker-status,
.sidebar:not(.is-collapsed) .version {
  transition-delay: 0.05s;
}

.sidebar.is-collapsed .logo-text, 
.sidebar.is-collapsed .label, 
.sidebar.is-collapsed .user-info,
.sidebar.is-collapsed .docker-status, 
.sidebar.is-collapsed .version {
  opacity: 0;
  visibility: hidden;
  pointer-events: none;
  transition: opacity 0.1s ease, visibility 0.1s, width 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  width: 0;
}

.sidebar.is-collapsed .logo {
  justify-content: center;
  padding: 0 0 2rem;
  gap: 0;
}

.sidebar.is-collapsed nav li {
  justify-content: center;
  padding: 0.6rem 0.5rem;
  gap: 0;
}

.sidebar.is-collapsed .user-profile {
  justify-content: center;
  padding: 0.75rem 0.5rem;
  gap: 0;
}

.logo {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0 0.5rem 2rem;
  color: #f0f6fc;
  font-weight: 600;
  transition: all 0.2s, padding 0.3s cubic-bezier(0.4, 0, 0.2, 1), justify-content 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  min-height: 2.5rem;
}

.logo.clickable {
  cursor: pointer;
}

.logo.clickable:hover {
  background-color: #161b22;
  border-radius: 6px;
}

.logo-icon {
  font-size: 1.5rem;
  color: #58a6ff;
  flex-shrink: 0;
  width: 24px;
  display: flex;
  justify-content: center;
  align-items: center;
}

nav ul {
  list-style: none;
  padding: 0;
  margin: 0;
}

nav li {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.6rem 0.75rem;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s, padding 0.3s cubic-bezier(0.4, 0, 0.2, 1), justify-content 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  margin-bottom: 0.25rem;
  font-size: 0.9rem;
}

nav li:hover {
  background-color: #161b22;
  color: #f0f6fc;
}

nav li.active {
  background-color: #1f6feb;
  color: #ffffff;
}

.icon {
  font-size: 1.1rem;
  width: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.spacer {
  flex-grow: 1;
}

.user-profile {
  background-color: #161b22;
  border-radius: 8px;
  padding: 0.75rem;
  display: flex;
  align-items: center;
  gap: 0.75rem;
  margin-bottom: 1rem;
  transition: padding 0.3s cubic-bezier(0.4, 0, 0.2, 1), justify-content 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  user-select: none;
}

.avatar {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
}

.avatar img {
  width: 32px;
  height: 32px;
  border-radius: 50%;
}

.user-info {
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.user-info .name {
  color: #f0f6fc;
  font-size: 0.85rem;
  font-weight: 600;
}

.user-info .email {
  font-size: 0.75rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.sidebar.is-collapsed .docker-status {
  justify-content: center;
  padding: 0;
  gap: 0;
  user-select: none;
}

.sidebar.is-collapsed .docker-status .status-dots {
  display: none;
}

.sidebar.is-collapsed .docker-info {
  justify-content: center;
  gap: 0;
}

.docker-status {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 0.5rem;
  margin-bottom: 0.5rem;
  min-height: 1.5rem;
}

.docker-info {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.85rem;
}

.status-dots {
  display: flex;
  gap: 0.25rem;
}

.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background-color: #30363d;
}

.dot.green {
  background-color: #3fb950;
  box-shadow: 0 0 5px #3fb950;
}

.dot.red {
  background-color: #f85149;
  box-shadow: 0 0 5px #f85149;
}

.version {
  font-size: 0.7rem;
  text-align: right;
  padding-right: 0.5rem;
  opacity: 0.5;
}
</style>
