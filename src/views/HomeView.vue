<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import { useSettingsStore } from "../stores/settings";
import { useWorkspacesStore } from "../stores/workspaces";

const router = useRouter();
const workspacesStore = useWorkspacesStore();
const settingsStore = useSettingsStore();

const totalServices = computed(() => {
  return workspacesStore.items.reduce((count, workspace) => count + workspace.services.length, 0);
});

onMounted(async () => {
  workspacesStore.clearSelectedWorkspace();
  await settingsStore.loadPersistedSettings();
  await workspacesStore.fetchWorkspaces();
});

function openWorkspaceDetail(workspaceId: string) {
  workspacesStore.selectWorkspace(workspaceId);
  settingsStore.setLastWorkspace(workspaceId);
  router.push({ name: "workspace-detail", params: { id: workspaceId } });
}

function openLastWorkspace() {
  if (!settingsStore.lastWorkspaceId) {
    return;
  }
  openWorkspaceDetail(settingsStore.lastWorkspaceId);
}
</script>

<template>
  <main class="home-container">
    <header class="hero panel">
      <p class="eyebrow">Bienvenue</p>
      <h1>Dev Workspace Manager</h1>
      <p class="subtitle">Centralisez le lancement et le suivi de vos environnements de développement.</p>
      <div class="hero-actions">
        <button
          class="primary"
          @click="openLastWorkspace"
          :disabled="!settingsStore.lastWorkspaceId"
          aria-label="Ouvrir le dernier workspace"
          title="Ouvrir le dernier workspace"
        >
          🕘
        </button>
        <button
          class="secondary"
          @click="router.push({ name: 'workspace-create' })"
          aria-label="Créer un workspace"
          title="Créer un workspace"
        >
          ➕
        </button>
      </div>
    </header>

    <section class="stats-grid">
      <article class="panel stat-card">
        <p class="stat-label">Workspaces</p>
        <p class="stat-value">{{ workspacesStore.items.length }}</p>
      </article>
      <article class="panel stat-card">
        <p class="stat-label">Services configurés</p>
        <p class="stat-value">{{ totalServices }}</p>
      </article>
      <article class="panel stat-card">
        <p class="stat-label">État UI</p>
        <p class="stat-value">{{ workspacesStore.items.length === 0 ? "--" : settingsStore.uiState }}</p>
      </article>
    </section>

    <section class="panel">
      <h2>Démarrer rapidement</h2>
      <p v-if="workspacesStore.isLoading">Chargement des workspaces...</p>
      <p v-else-if="workspacesStore.error" class="error">{{ workspacesStore.error }}</p>
      <p v-else-if="workspacesStore.items.length === 0">Aucun workspace enregistré pour le moment.</p>
      <ul v-else class="workspace-grid">
        <li v-for="workspace in workspacesStore.items" :key="workspace.id" class="workspace-card">
          <h3>{{ workspace.name }}</h3>
          <p>{{ workspace.root }}</p>
          <p v-if="workspace.services.length > 0" class="workspace-services">
            Services : {{ workspace.services.map((service) => service.name).join(", ") }}
          </p>
          <button
            class="link-button"
            @click="openWorkspaceDetail(workspace.id)"
            aria-label="Voir le détail du workspace"
            title="Voir le détail du workspace"
          >
            🔍
          </button>
        </li>
      </ul>
    </section>
  </main>
</template>

<style scoped>
.home-container {
  max-width: 1200px;
  margin: 0 auto;
  padding: 2rem 1.25rem 2.5rem;
  color: #14213d;
}

.panel {
  border: 1px solid #d9e2f1;
  border-radius: 14px;
  background: #fff;
  box-shadow: 0 10px 24px rgba(17, 24, 39, 0.06);
}

.hero {
  padding: 1.6rem;
  background: linear-gradient(135deg, #f7faff, #eef4ff);
  margin-bottom: 1.25rem;
}

.eyebrow {
  margin: 0;
  font-size: 0.78rem;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: #4f5d75;
}

h1 {
  margin: 0.2rem 0 0;
}

.subtitle {
  margin: 0.45rem 0 0;
  color: #4f5d75;
}

.hero-actions {
  display: flex;
  gap: 0.6rem;
  margin-top: 1rem;
}

button {
  border: 1px solid #396cd8;
  border-radius: 10px;
  padding: 0.58rem 0.85rem;
  font-weight: 600;
  cursor: pointer;
}

.primary {
  background: #396cd8;
  color: #fff;
}

.secondary {
  background: #fff;
  color: #396cd8;
}

button:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 1rem;
  margin-bottom: 1rem;
}

.stat-card {
  padding: 1rem 1.1rem;
}

.stat-label {
  margin: 0;
  color: #4f5d75;
  font-size: 0.86rem;
}

.stat-value {
  margin: 0.35rem 0 0;
  font-size: 1.5rem;
  font-weight: 700;
}

section.panel {
  padding: 1.2rem;
  margin-bottom: 1rem;
}

.workspace-grid {
  list-style: none;
  padding: 0;
  margin: 0.75rem 0 0;
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.9rem;
}

.workspace-card {
  border: 1px solid #dce7fa;
  border-radius: 12px;
  padding: 0.9rem;
  background: #f8fbff;
}

.workspace-card h3 {
  margin-top: 0;
  margin-bottom: 0.35rem;
}

.workspace-card p {
  margin: 0 0 0.75rem;
  color: #4f5d75;
  font-size: 0.9rem;
}

.workspace-services {
  font-weight: 600;
}

.link-button {
  background: #fff;
  color: #396cd8;
}

.error {
  color: #c40000;
  font-weight: 600;
}

@media (max-width: 900px) {
  .stats-grid,
  .workspace-grid {
    grid-template-columns: 1fr;
  }
}
</style>
