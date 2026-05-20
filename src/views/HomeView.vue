<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import { useRuntimeStore } from "../stores/runtime";
import { useSettingsStore } from "../stores/settings";
import { useWorkspacesStore } from "../stores/workspaces";
import StatCard from "../components/dashboard/StatCard.vue";
import WorkspaceCard from "../components/dashboard/WorkspaceCard.vue";
import ServiceTable from "../components/dashboard/ServiceTable.vue";
import RightPanel from "../components/dashboard/RightPanel.vue";

const router = useRouter();
const workspacesStore = useWorkspacesStore();
const runtimeStore = useRuntimeStore();
const settingsStore = useSettingsStore();

const totalServices = computed(() => {
  return workspacesStore.items.reduce((count, workspace) => count + workspace.services.length, 0);
});

const activeServices = computed(() => {
    let count = 0;
    for (const wsId in runtimeStore.byWorkspaceId) {
        count += runtimeStore.byWorkspaceId[wsId].services.filter(s => s.status === 'running').length;
    }
    return count;
});

const activeWorkspaces = computed(() => {
    return workspacesStore.items.filter(ws => getWorkspaceGlobalStatus(ws.id) === 'running').length;
});

const totalContainers = computed(() => {
  const containerNames = new Set<string>();
  workspacesStore.items.forEach(workspace => {
    workspace.services.forEach(service => {
      containerNames.add(service.name);
    });
  });
  return containerNames.size;
});

const activeContainers = computed(() => {
  const runningContainerNames = new Set<string>();
  for (const wsId in runtimeStore.byWorkspaceId) {
    const wsState = runtimeStore.byWorkspaceId[wsId];
    wsState.services.forEach(service => {
      if (service.status === 'running') {
        runningContainerNames.add(service.name);
      }
    });
  }
  return runningContainerNames.size;
});

const serviceData = computed(() => {
  const allServices = [];
  for (const workspace of workspacesStore.items) {
    const wsState = runtimeStore.byWorkspaceId[workspace.id];
    if (!wsState) continue;
    
    for (const service of workspace.services) {
      const sState = wsState.services.find(s => s.name === service.name);
      allServices.push({
        config: service,
        workspace: workspace,
        status: sState?.status || 'idle',
        // On simule des ports pour le design
        port: sState?.status === 'running' ? 3000 + Math.floor(Math.random() * 5000) : undefined,
        lastActivity: sState?.last_transition ? formatTimeAgo(sState.last_transition) : 'Jamais utilisé'
      });
    }
  }
  
  // Trier par activité récente
  return allServices
    .sort((a, b) => {
        const wsA = runtimeStore.byWorkspaceId[a.workspace.id];
        const wsB = runtimeStore.byWorkspaceId[b.workspace.id];
        const sA = wsA?.services.find(s => s.name === a.config.name);
        const sB = wsB?.services.find(s => s.name === b.config.name);
        return (sB?.last_transition || 0) - (sA?.last_transition || 0);
    })
    .slice(0, 6);
});

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
  workspacesStore.clearSelectedWorkspace();
  await settingsStore.loadPersistedSettings();
  await workspacesStore.fetchWorkspaces();
  
  for (const workspace of workspacesStore.items) {
    await runtimeStore.refreshWorkspaceState(workspace.id);
  }

  // Recharger les activités après les rafraîchissements d'état initiaux
  await runtimeStore.loadRecentRuns();
});

function getWorkspaceGlobalStatus(workspaceId: string) {
  return runtimeStore.byWorkspaceId[workspaceId]?.global_status ?? "idle";
}

function openWorkspaceDetail(workspaceId: string) {
  workspacesStore.selectWorkspace(workspaceId);
  settingsStore.setLastWorkspace(workspaceId);
  router.push({ name: "workspace-detail", params: { id: workspaceId } });
}

function createNewWorkspace() {
    router.push({ name: 'workspace-create' });
}
</script>

<template>
  <div class="dashboard">
    <div class="main-content">
      <header class="dashboard-header">
        <div class="welcome">
          <h1>Bonjour {{ settingsStore.username.split(' ')[0] }} ! 👋</h1>
          <p class="subtitle">Gérez tous vos workspaces et leurs services depuis un seul endroit.</p>
        </div>
        <div class="header-actions">
          <button class="btn-primary" @click="createNewWorkspace">
            <span class="icon">＋</span> Nouveau workspace
          </button>
          <div class="header-icons">
            <button class="icon-btn">🔍</button>
            <button class="icon-btn badge">🔔<span class="count">3</span></button>
            <button class="icon-btn">⚙️</button>
          </div>
        </div>
      </header>

      <div class="stats-row">
        <StatCard 
          title="Workspaces" 
          :value="workspacesStore.items.length" 
          :subValue="activeWorkspaces + ' en cours d\'exécution'" 
          icon="🟦" 
          color="#58a6ff" 
        />
        <StatCard 
          title="Services" 
          :value="totalServices" 
          :subValue="activeServices + ' en cours'" 
          icon="🟩" 
          color="#3fb950" 
        />
        <!-- TODO: Ne pas supprimer, sera utilisé plus tard -->
        <!--
        <StatCard 
          title="Environnements" 
          :value="6" 
          subValue="4 actifs" 
          icon="🟪" 
          color="#bc8cff" 
        />
        -->
        <StatCard 
          title="Conteneurs" 
          :value="totalContainers" 
          :subValue="activeContainers + ' en cours'" 
          icon="🟨" 
          color="#d29922" 
        />
      </div>

      <section class="section">
        <div class="section-header">
          <h2>Workspaces récents</h2>
          <a href="#" class="view-all">Voir tous</a>
        </div>
        <div class="workspace-grid">
          <WorkspaceCard 
            v-for="ws in workspacesStore.items.slice(0, 4)" 
            :key="ws.id" 
            :workspace="ws" 
            :status="getWorkspaceGlobalStatus(ws.id)"
            @click="openWorkspaceDetail"
          />
        </div>
      </section>

      <section class="section">
        <div class="section-header">
          <h2>Aperçu des services</h2>
          <div class="filters">
             <select><option>Tous les workspaces</option></select>
             <select><option>Trier par statut</option></select>
          </div>
        </div>
        <ServiceTable :services="serviceData" />
      </section>

      <section class="section">
        <h2>⭐️ Workspaces favoris</h2>
        <div class="favorites-row">
          <div v-for="ws in workspacesStore.items.slice(0, 3)" :key="'fav-'+ws.id" class="fav-tag">
             {{ ws.name }}
          </div>
          <button class="add-fav">＋ Ajouter aux favoris</button>
        </div>
      </section>
    </div>

    <RightPanel />
  </div>
</template>

<style scoped>
.dashboard {
  display: flex;
  padding: 2rem;
  gap: 2rem;
  width: 100%;
  max-width: 1600px;
  margin: 0 auto;
}

.main-content {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2.5rem;
}

.dashboard-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
}

.welcome h1 {
  font-size: 1.8rem;
  margin-bottom: 0.5rem;
  color: #f0f6fc;
}

.subtitle {
  color: #8b949e;
  font-size: 0.95rem;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 1.5rem;
  flex-wrap: wrap;
}

.btn-primary {
  background-color: #1f6feb;
  color: white;
  border: none;
  padding: 0.6rem 1.2rem;
  border-radius: 8px;
  font-weight: 600;
  display: flex;
  align-items: center;
  gap: 0.5rem;
  cursor: pointer;
}

.header-icons {
  display: flex;
  gap: 0.5rem;
}

.icon-btn {
  background-color: #161b22;
  border: 1px solid #30363d;
  color: #f0f6fc;
  width: 40px;
  height: 40px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  position: relative;
}

.icon-btn.badge .count {
  position: absolute;
  top: -5px;
  right: -5px;
  background-color: #f85149;
  color: white;
  font-size: 0.7rem;
  padding: 2px 5px;
  border-radius: 10px;
  border: 2px solid #0d1117;
}

.stats-row {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  gap: 1rem;
}

@media (max-width: 600px) {
  .stats-row {
    grid-template-columns: 1fr;
  }
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1.25rem;
  gap: 1rem;
  flex-wrap: wrap;
}

.filters {
  display: flex;
  gap: 0.75rem;
  flex-wrap: wrap;
}

.section h2 {
  font-size: 1.1rem;
  color: #f0f6fc;
  font-weight: 600;
}

.view-all {
  color: #58a6ff;
  font-size: 0.85rem;
  text-decoration: none;
}

.workspace-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
  gap: 1rem;
}

.filters {
  display: flex;
  gap: 1rem;
  flex-wrap: wrap;
}

.filters select {
  background-color: #161b22;
  border: 1px solid #30363d;
  color: #8b949e;
  padding: 0.4rem 0.8rem;
  border-radius: 6px;
  font-size: 0.85rem;
}

.favorites-row {
  display: flex;
  align-items: center;
  gap: 1rem;
  margin-top: 1rem;
}

.fav-tag {
  background-color: #161b22;
  border: 1px solid #30363d;
  padding: 0.5rem 1rem;
  border-radius: 8px;
  font-size: 0.9rem;
  color: #f0f6fc;
}

.add-fav {
  background: none;
  border: 1px dashed #30363d;
  color: #8b949e;
  padding: 0.5rem 1rem;
  border-radius: 8px;
  font-size: 0.9rem;
  cursor: pointer;
}

@media (max-width: 1200px) {
  .dashboard {
    flex-direction: column;
    padding: 1rem;
  }
}

@media (max-width: 768px) {
  .dashboard-header {
    flex-direction: column;
    gap: 1.5rem;
  }
  
  .header-actions {
    width: 100%;
    justify-content: space-between;
    gap: 1rem;
  }

  .section-header {
    flex-direction: column;
    align-items: flex-start;
  }

  .filters {
    width: 100%;
  }

  .filters select {
    flex-grow: 1;
    min-width: 150px;
  }
}
</style>
