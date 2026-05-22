<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { computed, onMounted, ref } from "vue";
import type { WorkspaceServiceImage } from "../types";

const images = ref<WorkspaceServiceImage[]>([]);
const isLoading = ref(false);
const error = ref<string | null>(null);
const currentPage = ref(1);
const pageSize = 10;

const hasImages = computed(() => images.value.length > 0);
const totalPages = computed(() => Math.max(1, Math.ceil(images.value.length / pageSize)));

const paginatedImages = computed(() => {
  const start = (currentPage.value - 1) * pageSize;
  return images.value.slice(start, start + pageSize);
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

onMounted(async () => {
  isLoading.value = true;
  error.value = null;

  try {
    images.value = await invoke<WorkspaceServiceImage[]>("list_workspace_service_images");
  } catch (loadError) {
    error.value = loadError instanceof Error ? loadError.message : "Impossible de charger les images";
  } finally {
    isLoading.value = false;
  }
});
</script>

<template>
  <div class="images-view">
    <header class="header">
      <h1>Images</h1>
      <p class="subtitle">Vue globale des images Docker.</p>
    </header>

    <section class="content">
      <p v-if="isLoading" class="state">Chargement des images...</p>
      <p v-else-if="error" class="state state-error">{{ error }}</p>

      <div v-else-if="hasImages" class="images-table-wrapper">
        <table class="images-table">
          <thead>
            <tr>
              <th>Image</th>
              <th>Service</th>
              <th>Workspace</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="item in paginatedImages" :key="`${item.workspace_id}-${item.service_name}-${item.image}`">
              <td>
                <div class="image-tag">
                  <a class="access-link" :href="`https://hub.docker.com/layers/library/${item.image.split(':')[0]}/${item.image.split(':')[1]}`" target="_blank" rel="noopener noreferrer">
                    {{ item.image }}
                  </a>
                </div>
              </td>
              <td>{{ item.service_name }}</td>
              <td>
                <router-link
                  class="workspace-link"
                  :to="{ name: 'workspace-detail', params: { id: item.workspace_id } }"
                >
                  {{ item.workspace_name }}
                </router-link>
              </td>
            </tr>
          </tbody>
        </table>

        <div class="pagination">
          <button class="btn-pagination" :disabled="currentPage === 1" @click="goToPreviousPage">Précédent</button>
          <span>Page {{ currentPage }} / {{ totalPages }} · {{ images.length }} image(s)</span>
          <button class="btn-pagination" :disabled="currentPage === totalPages" @click="goToNextPage">Suivant</button>
        </div>
      </div>

      <div v-else class="empty-state">
        <p>Aucune image à afficher pour le moment.</p>
      </div>
    </section>
  </div>
</template>

<style scoped>
.images-view {
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

.images-table-wrapper {
  display: flex;
  flex-direction: column;
  gap: 0.8rem;
}

.images-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.9rem;
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 10px;
  overflow: hidden;
}

.images-table th,
.images-table td {
  padding: 0.8rem;
  border-bottom: 1px solid #30363d;
  text-align: left;
}

.images-table th {
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

.image-tag {
  background-color: #0d1117;
  padding: 0.25rem 0.5rem;
  border-radius: 6px;
  font-size: 0.75rem;
  border: 1px solid #30363d;
  display: inline-block;
  font-family: monospace;
  color: #8b949e;
}

.access-link {
  color: #79c0ff;
  text-decoration: none;
}

.access-link:visited {
  color: #79c0ff;
}

.access-link:hover {
  text-decoration: underline;
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