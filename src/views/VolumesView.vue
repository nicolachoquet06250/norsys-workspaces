<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { computed, ref } from "vue";
import { useVolumesStore } from "../stores/volumes";

const volumesStore = useVolumesStore();
const actionError = ref<string | null>(null);
const currentPage = ref(1);
const pageSize = 10;

const hasVolumes = computed(() => volumesStore.hasVolumes);
const totalPages = computed(() => Math.max(1, Math.ceil(volumesStore.items.length / pageSize)));
const isLoading = computed(() => volumesStore.isLoading);
const error = computed(() => volumesStore.error);

const paginatedVolumes = computed(() => {
  const start = (currentPage.value - 1) * pageSize;
  return volumesStore.items.slice(start, start + pageSize);
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
    if (!path.startsWith('/mnt/')) {
      // const defaultWslDistrib = await invoke<string>('get_default_wsl_distro');
      // path = `\\\\wsl$\\${defaultWslDistrib}${path}`;
      path = path.replace(/\\/g, '/');
      console.log(path);
      return
    }
    await invoke("open_path_in_file_manager", { path });
    console.log(path);
  } catch (openError) {
    actionError.value = openError instanceof Error ? openError.message : "Impossible d'ouvrir le répertoire";
  }
}

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

        <div class="volumes-table-scroll">
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
                  <template v-if="item.host_path">
                    <button
                        v-if="item.host_path.startsWith('C:\\')"
                        class="link-btn"
                        @click="openVolumeDirectory(item.host_path)"
                    >
                      Ouvrir le dossier
                    </button>
                    <span v-else>{{ item.host_path.replace(/\\/g, '/') }}</span>
                  </template>
                  <span v-else class="muted">N/A</span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <div class="pagination">
          <button class="btn-pagination" :disabled="currentPage === 1" @click="goToPreviousPage">Précédent</button>
          <span>Page {{ currentPage }} / {{ totalPages }} · {{ volumesStore.items.length }} volume(s)</span>
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
  height: 100vh;
  padding: 2rem;
  background: #0d1117;
  color: #f0f6fc;
  overflow: hidden;
  display: flex;
  flex-direction: column;
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
  min-height: 0;
}

.content {
  min-height: 0;
}

.volumes-table-scroll {
  overflow: auto;
  min-height: 0;
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