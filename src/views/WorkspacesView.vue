<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import { useWorkspacesStore } from "../stores/workspaces";

const router = useRouter();
const workspacesStore = useWorkspacesStore();

const hasWorkspaces = computed(() => workspacesStore.items.length > 0);

onMounted(async () => {
  workspacesStore.clearSelectedWorkspace();
  await workspacesStore.fetchWorkspaces();
});

function openWorkspace(workspaceId: string) {
  workspacesStore.selectWorkspace(workspaceId);
  router.push({ name: "workspace-detail", params: { id: workspaceId } });
}
</script>

<template>
  <div class="workspaces-view">
    <header class="header">
      <div>
        <h1>Workspaces</h1>
        <p class="subtitle">Liste de vos espaces de travail disponibles.</p>
      </div>
      <button class="btn-primary" @click="router.push({ name: 'workspace-create' })">Nouveau workspace</button>
    </header>

    <section class="content">
      <p v-if="workspacesStore.isLoading" class="state">Chargement des workspaces...</p>
      <p v-else-if="workspacesStore.error" class="state state-error">{{ workspacesStore.error }}</p>

      <div v-else-if="hasWorkspaces" class="workspace-grid">
        <article
          v-for="workspace in workspacesStore.items"
          :key="workspace.id"
          class="workspace-card"
          @click="openWorkspace(workspace.id)"
        >
          <h2>{{ workspace.name }}</h2>
          <p class="path">{{ workspace.root }}</p>
          <p class="meta">{{ workspace.services.length }} service(s)</p>
        </article>
      </div>

      <div v-else class="empty-state">
        <p>Aucun workspace disponible pour le moment.</p>
      </div>
    </section>
  </div>
</template>

<style scoped>
.workspaces-view {
  min-height: 100vh;
  padding: 2rem;
  background: #0d1117;
  color: #f0f6fc;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 1rem;
  margin-bottom: 1.5rem;
}

h1 {
  font-size: 1.8rem;
}

.subtitle {
  color: #8b949e;
  margin-top: 0.35rem;
}

.btn-primary {
  border: none;
  border-radius: 8px;
  padding: 0.65rem 1rem;
  background: #238636;
  color: #ffffff;
  cursor: pointer;
  font-weight: 600;
}

.btn-primary:hover {
  background: #2ea043;
}

.workspace-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 1rem;
}

.workspace-card {
  padding: 1rem;
  border-radius: 10px;
  border: 1px solid #30363d;
  background: #161b22;
  cursor: pointer;
}

.workspace-card:hover {
  border-color: #58a6ff;
}

.path {
  margin-top: 0.5rem;
  color: #8b949e;
  font-size: 0.9rem;
  word-break: break-all;
}

.meta {
  margin-top: 0.75rem;
  color: #58a6ff;
  font-size: 0.85rem;
}

.state,
.empty-state {
  padding: 1rem;
  border: 1px solid #30363d;
  border-radius: 8px;
  background: #161b22;
}

.state-error {
  color: #f85149;
}
</style>