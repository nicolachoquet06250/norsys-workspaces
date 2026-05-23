<script setup lang="ts">
import { ref } from "vue";
import Sidebar from "../components/layout/Sidebar.vue";

const isSidebarCollapsed = ref(window.innerWidth <= 768);
const isMobile = ref(window.innerWidth <= 768);

window.addEventListener("resize", () => {
  isMobile.value = window.innerWidth <= 768;
  if (window.innerWidth > 768) {
    isSidebarCollapsed.value = false;
  } else if (!isSidebarCollapsed.value && isMobile.value) {
    isSidebarCollapsed.value = true;
  }
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
    <div v-if="!isSidebarCollapsed && isMobile" class="sidebar-overlay" @click="closeSidebar" />
    <Sidebar :is-collapsed="isSidebarCollapsed" @toggle="toggleSidebar" />

    <main class="app-main" @click="closeSidebar">
      <RouterView />
    </main>
  </div>
</template>

<style scoped>
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
</style>