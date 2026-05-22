<script setup lang="ts">
import { computed, ref } from "vue";
import { useNetworksStore } from "../stores/networks";

const networksStore = useNetworksStore();
const currentPage = ref(1);
const pageSize = 10;

const hasNetworks = computed(() => networksStore.hasNetworks);
const totalPages = computed(() => Math.max(1, Math.ceil(networksStore.items.length / pageSize)));
const isLoading = computed(() => networksStore.isLoading);
const error = computed(() => networksStore.error);

const paginatedNetworks = computed(() => {
  const start = (currentPage.value - 1) * pageSize;
  return networksStore.items.slice(start, start + pageSize);
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

</script>

<template>
  <div class="networks-view">
    <header class="header">
      <h1>Networks</h1>
      <p class="subtitle">Vue globale des réseaux utilisés par vos services.</p>
    </header>

    <section class="content">
      <p v-if="isLoading" class="state">Chargement des réseaux...</p>
      <p v-else-if="error" class="state state-error">{{ error }}</p>

      <div v-else-if="hasNetworks" class="networks-table-wrapper">
        <div class="networks-table-scroll">
          <table class="networks-table">
            <thead>
              <tr>
                <th>Réseau</th>
                <th>Service</th>
                <th>Workspace</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="item in paginatedNetworks" :key="`${item.workspace_id}-${item.service_name}-${item.network}`">
                <td>{{ item.network }}</td>
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
        </div>

        <div class="pagination">
          <button class="btn-pagination" :disabled="currentPage === 1" @click="goToPreviousPage">Précédent</button>
          <span>Page {{ currentPage }} / {{ totalPages }} · {{ networksStore.items.length }} réseau(x)</span>
          <button class="btn-pagination" :disabled="currentPage === totalPages" @click="goToNextPage">Suivant</button>
        </div>
      </div>

      <div v-else class="empty-state">
        <p>Aucun réseau à afficher pour le moment.</p>
      </div>
    </section>
  </div>
</template>

<style scoped>
.networks-view {
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

.networks-table-wrapper {
  display: flex;
  flex-direction: column;
  gap: 0.8rem;
  min-height: 0;
}

.content {
  min-height: 0;
}

.networks-table-scroll {
  overflow: auto;
  min-height: 0;
}

.networks-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.9rem;
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 10px;
  overflow: hidden;
}

.networks-table th,
.networks-table td {
  padding: 0.8rem;
  border-bottom: 1px solid #30363d;
  text-align: left;
}

.networks-table th {
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
