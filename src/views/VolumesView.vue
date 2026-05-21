<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { computed, onMounted, onUnmounted, ref } from "vue";
import type { WorkspaceServiceVolume } from "../types";

const volumes = ref<WorkspaceServiceVolume[]>([]);
const isLoading = ref(false);
const error = ref<string | null>(null);
const actionError = ref<string | null>(null);
const currentPage = ref(1);
const pageSize = 10;
let refreshHandle: number | null = null;

const hasVolumes = computed(() => volumes.value.length > 0);
const totalPages = computed(() => Math.max(1, Math.ceil(volumes.value.length / pageSize)));

const paginatedVolumes = computed(() => {
  const start = (currentPage.value - 1) * pageSize;
  return volumes.value.slice(start, start + pageSize);
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

async function openVolumeDirectory(path: string | null) {
  if (!path) {
    return;
  }

  actionError.value = null;
  try {
    path = path.replace(/.\//, '');
    await invoke("open_path_in_file_manager", { path });
    console.log(path);
  } catch (openError) {
    actionError.value = openError instanceof Error ? openError.message : "Impossible d'ouvrir le répertoire";
  }
}

async function loadVolumes() {
  isLoading.value = true;
  error.value = null;

  try {
    volumes.value = await invoke<WorkspaceServiceVolume[]>("list_workspace_service_volumes");
  } catch (loadError) {
    error.value = loadError instanceof Error ? loadError.message : "Impossible de charger les volumes";
  } finally {
    isLoading.value = false;
  }
}

onMounted(async () => {
  await loadVolumes();
  refreshHandle = window.setInterval(() => {
    void loadVolumes();
  }, 2000);
});

onUnmounted(() => {
  if (refreshHandle !== null) {
    window.clearInterval(refreshHandle);
    refreshHandle = null;
  }
});
</script>

<template>
  <div class="volumes-view">
    <header class="header">
      <h1>Volumes</h1>
      <p class="subtitle">Vue globale des volumes utilisés par vos services.</p>
    </header>

    <section class="content">
      <p v-if="isLoading" class="state">Chargement des volumes...</p>
      <p v-else-if="error" class="state state-error">{{ error }}</p>

      <div v-else-if="hasVolumes" class="volumes-table-wrapper">
        <p v-if="actionError" class="state state-error">{{ actionError }}</p>

        <table class="volumes-table">
          <thead>
            <tr>
              <th>Volume</th>
              <th>Service</th>
              <th>Workspace</th>
              <th>Répertoire</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="item in paginatedVolumes" :key="`${item.workspace_id}-${item.service_name}-${item.volume}-${item.host_path}`">
              <td>{{ item.volume }}</td>
              <td>{{ item.service_name }}</td>
              <td>
                <router-link
                  class="workspace-link"
                  :to="{ name: 'workspace-detail', params: { id: item.workspace_id } }"
                >
                  {{ item.workspace_name }}
                </router-link>
              </td>
              <td>
                <button
                  v-if="item.host_path"
                  class="link-btn"
                  @click="openVolumeDirectory(item.host_path)"
                >
                  Ouvrir le dossier
                </button>
                <span v-else class="muted">N/A</span>
              </td>
            </tr>
          </tbody>
        </table>

        <div class="pagination">
          <button class="btn-pagination" :disabled="currentPage === 1" @click="goToPreviousPage">Précédent</button>
          <span>Page {{ currentPage }} / {{ totalPages }} · {{ volumes.length }} volume(s)</span>
          <button class="btn-pagination" :disabled="currentPage === totalPages" @click="goToNextPage">Suivant</button>
        </div>
      </div>

      <div v-else class="empty-state">
        <p>Aucun volume à afficher pour le moment.</p>
      </div>
    </section>
  </div>
</template>

<style scoped>
.volumes-view {
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

.volumes-table-wrapper {
  display: flex;
  flex-direction: column;
  gap: 0.8rem;
}

.volumes-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.9rem;
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 10px;
  overflow: hidden;
}

.volumes-table th,
.volumes-table td {
  padding: 0.8rem;
  border-bottom: 1px solid #30363d;
  text-align: left;
}

.volumes-table th {
  color: #8b949e;
  font-weight: 500;
}

.workspace-link {
  color: #79c0ff;
  text-decoration: none;
}

.workspace-link:visited {
  color: #79c0ff;
}

.workspace-link:hover {
  text-decoration: underline;
}

.link-btn {
  border: none;
  background: transparent;
  color: #79c0ff;
  text-decoration: underline;
  padding: 0;
  cursor: pointer;
}

.muted {
  color: #8b949e;
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