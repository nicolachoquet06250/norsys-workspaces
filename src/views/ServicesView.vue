<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRuntimeStore } from "../stores/runtime";
import { useWorkspacesStore } from "../stores/workspaces";
import ServiceTable from "../components/dashboard/ServiceTable.vue";

const workspacesStore = useWorkspacesStore();
const runtimeStore = useRuntimeStore();
const currentPage = ref(1);
const pageSize = 10;

const servicesData = computed(() => {
  const allServices = [];

  for (const workspace of workspacesStore.items) {
    const workspaceState = runtimeStore.byWorkspaceId[workspace.id];

    for (const service of workspace.services) {
      const runtimeService = workspaceState?.services.find((item) => item.name === service.name);
      allServices.push({
        config: service,
        workspace,
        status: runtimeService?.status ?? "idle",
        port: runtimeService?.status === "running" ? 3000 + Math.floor(Math.random() * 5000) : undefined,
        lastActivity: runtimeService?.last_transition ? formatTimeAgo(runtimeService.last_transition) : "Jamais utilisé",
      });
    }
  }

  return allServices.sort((a, b) => {
    const aState = runtimeStore.byWorkspaceId[a.workspace.id]?.services.find((item) => item.name === a.config.name);
    const bState = runtimeStore.byWorkspaceId[b.workspace.id]?.services.find((item) => item.name === b.config.name);
    return (bState?.last_transition ?? 0) - (aState?.last_transition ?? 0);
  });
});

const hasServices = computed(() => servicesData.value.length > 0);
const totalPages = computed(() => Math.max(1, Math.ceil(servicesData.value.length / pageSize)));

const paginatedServices = computed(() => {
  const start = (currentPage.value - 1) * pageSize;
  return servicesData.value.slice(start, start + pageSize);
});

watch(servicesData, () => {
  if (currentPage.value > totalPages.value) {
    currentPage.value = totalPages.value;
  }
});

function goToPreviousPage() {
  if (currentPage.value > 1) {
    currentPage.value -= 1;
  }
}

function goToNextPage() {
  if (currentPage.value < totalPages.value) {
    currentPage.value += 1;
  }
}

function formatTimeAgo(timestamp: number): string {
  const seconds = Math.floor((Date.now() - timestamp) / 1000);
  if (seconds < 60) return `Il y a ${seconds} s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `Il y a ${minutes} min`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `Il y a ${hours} h`;
  return `Il y a ${Math.floor(hours / 24)} j`;
}

onMounted(async () => {
  await workspacesStore.fetchWorkspaces();
  await Promise.all(workspacesStore.items.map((workspace) => runtimeStore.refreshWorkspaceState(workspace.id)));
});
</script>

<template>
  <div class="services-view">
    <header class="header">
      <h1>Services</h1>
      <p class="subtitle">Vue globale de tous les services de vos workspaces.</p>
    </header>

    <section class="content">
      <p v-if="workspacesStore.isLoading" class="state">Chargement des services...</p>
      <p v-else-if="workspacesStore.error" class="state state-error">{{ workspacesStore.error }}</p>

      <div v-else-if="hasServices" class="services-table-wrapper">
        <ServiceTable :services="paginatedServices" :show-view-all-link="false" />

        <div class="pagination">
          <button class="btn-pagination" :disabled="currentPage === 1" @click="goToPreviousPage">Précédent</button>
          <span>Page {{ currentPage }} / {{ totalPages }} · {{ servicesData.length }} service(s)</span>
          <button class="btn-pagination" :disabled="currentPage === totalPages" @click="goToNextPage">Suivant</button>
        </div>
      </div>

      <div v-else class="empty-state">
        <p>Aucun service trouvé. Créez un workspace pour commencer.</p>
      </div>
    </section>
  </div>
</template>

<style scoped>
.services-view {
  min-height: 100vh;
  padding: 2rem;
  background: #0d1117;
  color: #f0f6fc;
}

.header {
  margin-bottom: 1.5rem;
}

h1 {
  font-size: 1.8rem;
}

.subtitle {
  color: #8b949e;
  margin-top: 0.35rem;
}

.state,
.empty-state {
  padding: 1rem;
  border: 1px solid #30363d;
  border-radius: 8px;
  background: #161b22;
}

.services-table-wrapper {
  display: flex;
  flex-direction: column;
  gap: 0.8rem;
}

.pagination {
  display: flex;
  justify-content: flex-end;
  align-items: center;
  gap: 0.75rem;
  color: #8b949e;
  font-size: 0.85rem;
}

.btn-pagination {
  border: 1px solid #30363d;
  border-radius: 8px;
  padding: 0.4rem 0.75rem;
  background: #161b22;
  color: #f0f6fc;
  cursor: pointer;
}

.btn-pagination:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.state-error {
  color: #f85149;
}
</style>