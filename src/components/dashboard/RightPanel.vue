<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from "vue";
import { useRouter } from "vue-router";
import { useRuntimeStore } from "../../stores/runtime";
import { useWorkspacesStore } from "../../stores/workspaces";
import { ServiceRuntimeStatus } from "../../types";

const runtimeStore = useRuntimeStore();
const workspacesStore = useWorkspacesStore();
const router = useRouter();

let statsInterval: any;
const rightPanelRef = ref<HTMLElement | null>(null);
const isSticky = ref(true);
let panelResizeObserver: ResizeObserver | null = null;

function updateStickyState() {
  if (!rightPanelRef.value) return;
  const panelHeight = rightPanelRef.value.getBoundingClientRect().height;
  isSticky.value = window.innerHeight > panelHeight;
}

onMounted(() => {
  runtimeStore.updateSystemStats();
  statsInterval = setInterval(() => {
    runtimeStore.updateSystemStats();
  }, 3000);

  void nextTick(() => {
    updateStickyState();

    if (rightPanelRef.value) {
      panelResizeObserver = new ResizeObserver(() => {
        updateStickyState();
      });
      panelResizeObserver.observe(rightPanelRef.value);
    }
  });

  window.addEventListener("resize", updateStickyState);
});

onUnmounted(() => {
  if (statsInterval) clearInterval(statsInterval);
  window.removeEventListener("resize", updateStickyState);
  panelResizeObserver?.disconnect();
  panelResizeObserver = null;
});

const resources = computed(() => {
  const stats = runtimeStore.systemStats;
  if (!stats) {
    return [
      { name: 'CPU', value: 0, label: 'chargement...', color: '#58a6ff' },
      { name: 'Mémoire', value: 0, label: 'chargement...', color: '#bc8cff' },
      { name: 'Disque', value: 0, label: 'chargement...', color: '#d29922' },
    ];
  }

  const memUsedGB = (stats.memory_used / 1024 / 1024 / 1024).toFixed(1);
  const memTotalGB = (stats.memory_total / 1024 / 1024 / 1024).toFixed(1);
  const memPercent = Math.round((stats.memory_used / stats.memory_total) * 100);

  const diskUsedGB = (stats.disk_used / 1024 / 1024 / 1024).toFixed(2);
  const diskTotalGB = (stats.disk_total / 1024 / 1024 / 1024).toFixed(2);
  const diskPercent = Math.round((stats.disk_used / stats.disk_total) * 100);

  return [
    { name: 'CPU', value: Math.round(stats.cpu_usage), label: `${Math.round(stats.cpu_usage)}%`, color: '#3fb950' },
    { name: 'Mémoire', value: memPercent, label: `${memUsedGB} / ${memTotalGB} GB`, color: '#bc8cff' },
    { name: 'Disque', value: diskPercent, label: `${diskUsedGB} / ${diskTotalGB} GB`, color: '#d29922' },
  ];
});

const actions = [
  { name: 'Nouveau workspace', icon: '➕', to: { name: 'workspace-create' } },
  { name: 'Parcourir les modèles', icon: '📋' },
  { name: 'Ouvrir le terminal', icon: '🖥️' },
  { name: 'Nettoyer les ressources', icon: '🧹' },
];

function handleAction(action: any) {
  if (action.to) {
    router.push(action.to);
  }
}

const activities = computed(() => {
  return runtimeStore.recentRuns.map(run => {
    const workspace = workspacesStore.items.find(ws => ws.id === run.workspace_id);
    let name;
    
    if (run.service_name === "_all_") {
      name = workspace?.name || run.workspace_id;
    } else {
      const service = workspace?.services.find(s => s.name === run.service_name);
      name = service?.display_name || run.service_name || workspace?.name || run.workspace_id;
    }
    
    return {
      name,
      action: formatAction(run.action, run.status),
      time: formatTimeAgo(parseInt(run.created_at) * 1000),
      icon: getActionIcon(run.action, run.status)
    };
  });
});

function formatAction(action: string, status: string): string {
  if (action.startsWith('status_change_to_')) {
    const newStatus = action.replace('status_change_to_', '');
    return formatStatus(newStatus as ServiceRuntimeStatus);
  }
  
  switch (action) {
    case 'start': return status === 'success' ? 'Démarré' : 'Échec démarrage';
    case 'stop': return 'Arrêté';
    default: return action;
  }
}

function getActionIcon(action: string, status: string): string {
  if (action.startsWith('status_change_to_')) {
    const newStatus = action.replace('status_change_to_', '');
    return getStatusIcon(newStatus as ServiceRuntimeStatus);
  }

  switch (action) {
    case 'start': return status === 'success' ? '🟢' : '🔴';
    case 'stop': return '⚪';
    default: return '❓';
  }
}

function formatStatus(status: ServiceRuntimeStatus): string {
  switch (status) {
    case 'running': return 'En cours';
    case 'starting': return 'Démarrage...';
    case 'stopping': return 'Arrêt...';
    case 'stopped': return 'Arrêté';
    case 'failed': return 'Échec';
    case 'blocked': return 'Bloqué';
    case 'idle': return 'Inactif';
    default: return status;
  }
}

function getStatusIcon(status: ServiceRuntimeStatus): string {
  switch (status) {
    case 'running': return '🟢';
    case 'starting': return '🔵';
    case 'stopping': return '🟠';
    case 'stopped': return '⚪';
    case 'failed': return '🔴';
    case 'blocked': return '🚫';
    case 'idle': return '💤';
    default: return '❓';
  }
}

function formatTimeAgo(timestamp: number): string {
  const seconds = Math.floor((Date.now() - timestamp) / 1000);
  
  if (seconds < 60) return `Il y a ${seconds} s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `Il y a ${minutes} min`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `Il y a ${hours} h`;
  const days = Math.floor(hours / 24);
  return `Il y a ${days} j`;
}
</script>

<template>
  <div ref="rightPanelRef" class="right-panel" :class="{ 'is-sticky': isSticky }">
    <section class="panel-section">
      <h3>Résumé rapide</h3>
      <div class="resource-list">
        <div v-for="res in resources" :key="res.name" class="resource-item">
          <div class="res-info">
            <span class="res-name">{{ res.name }}</span>
            <span class="res-label">{{ res.label }}</span>
          </div>
          <div class="progress-bar">
            <div class="progress-fill" :style="{ width: res.value + '%', backgroundColor: res.color }"></div>
          </div>
        </div>
      </div>
    </section>

    <section class="panel-section">
      <h3>Actions rapides</h3>
      <div class="action-list">
        <div 
          v-for="action in actions" 
          :key="action.name" 
          class="action-item"
          @click="handleAction(action)"
        >
          <span class="action-icon">{{ action.icon }}</span>
          <span class="action-name">{{ action.name }}</span>
          <span class="action-arrow">›</span>
        </div>
      </div>
    </section>

    <section class="panel-section">
      <div class="section-header">
        <h3>Activité récente</h3>
        <a href="#" class="view-all">Tout voir</a>
      </div>
      <div class="activity-list" v-if="activities.length > 0">
        <div v-for="act in activities" :key="act.name + act.time" class="activity-item">
          <div class="act-icon">{{ act.icon }}</div>
          <div class="act-content">
            <div class="act-top">
              <span class="act-name">{{ act.name }}</span>
              <span class="act-time">{{ act.time }}</span>
            </div>
            <div class="act-action">{{ act.action }}</div>
          </div>
        </div>
      </div>
      <div v-else class="empty-activity">
        <p>Aucune activité récente pour le moment.</p>
      </div>
    </section>
  </div>
</template>

<style scoped>
.right-panel {
  width: 300px;
  display: flex;
  flex-direction: column;
  gap: 2rem;
  padding-left: 1rem;
  /* min-height: 100vh; */
  height: min-content;
}

.right-panel.is-sticky {
  position: sticky;
  top: 35px;
}

@media (max-width: 1200px) {
  .right-panel {
    width: 100%;
    padding-left: 0;
    flex-direction: row;
    flex-wrap: wrap;
    gap: 1.5rem;
  }

  .panel-section {
    flex: 1 1 300px;
  }
}

@media (max-width: 768px) {
  .right-panel {
    flex-direction: column;
  }
}

.panel-section h3 {
  color: #f0f6fc;
  font-size: 0.95rem;
  margin-bottom: 1.25rem;
  font-weight: 600;
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1.25rem;
}

.section-header h3 {
  margin-bottom: 0;
}

.view-all {
  color: #58a6ff;
  font-size: 0.75rem;
  text-decoration: none;
}

/* Resources */
.resource-list {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.res-info {
  display: flex;
  justify-content: space-between;
  font-size: 0.8rem;
  margin-bottom: 0.4rem;
}

.res-name { color: #8b949e; }
.res-label { color: #f0f6fc; }

.progress-bar {
  height: 6px;
  background-color: #21262d;
  border-radius: 3px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  border-radius: 3px;
  transition: width 0.5s ease-in-out;
}

/* Actions */
.action-list {
  display: flex;
  flex-direction: column;
  background-color: #161b22;
  border: 1px solid #30363d;
  border-radius: 12px;
  overflow: hidden;
}

.action-item {
  display: flex;
  align-items: center;
  padding: 0.75rem 1rem;
  gap: 0.75rem;
  cursor: pointer;
  transition: background-color 0.2s;
  border-bottom: 1px solid #21262d;
}

.action-item:last-child {
  border-bottom: none;
}

.action-item:hover {
  background-color: #1c2128;
}

.action-icon { font-size: 1rem; }
.action-name { flex-grow: 1; font-size: 0.85rem; color: #f0f6fc; }
.action-arrow { color: #8b949e; }

/* Activity */
.activity-list {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.activity-item {
  display: flex;
  gap: 0.75rem;
}

.act-content {
  flex-grow: 1;
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
}

.act-top {
  display: flex;
  justify-content: space-between;
}

.act-name { color: #f0f6fc; font-size: 0.85rem; font-weight: 500; }
.act-time { color: #8b949e; font-size: 0.75rem; }
.act-action { color: #8b949e; font-size: 0.8rem; }

.empty-activity {
  padding: 1rem;
  background-color: #161b22;
  border: 1px dashed #30363d;
  border-radius: 8px;
  text-align: center;
}

.empty-activity p {
  color: #8b949e;
  font-size: 0.85rem;
  margin: 0;
}
</style>
