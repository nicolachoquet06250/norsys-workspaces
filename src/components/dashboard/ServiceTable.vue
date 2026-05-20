<script setup lang="ts">
import type { WorkspaceConfig, ServiceConfig, ServiceRuntimeStatus } from "../../types";

interface ServiceItem {
  config: ServiceConfig;
  workspace: WorkspaceConfig;
  status: ServiceRuntimeStatus;
  port?: number;
  lastActivity?: string;
}

withDefaults(defineProps<{
  services: ServiceItem[];
  showWorkspace?: boolean;
}>(), {
  showWorkspace: true
});

function getTechIcon(tech: string) {
  const t = tech.toLowerCase();
  if (t.includes('node') || t.includes('js')) return '🟢';
  if (t.includes('react')) return '🔵';
  if (t.includes('postgres') || t.includes('sql')) return '🐘';
  if (t.includes('redis')) return '🟥';
  if (t.includes('python')) return '🐍';
  return '📦';
}

function getStatusClass(status: ServiceRuntimeStatus) {
  return status;
}

function getStatusLabel(status: ServiceRuntimeStatus) {
  switch (status) {
    case 'running': return 'En cours';
    case 'starting': return 'Démarrage';
    case 'stopping': return 'Arrêt';
    case 'stopped': return 'Arrêté';
    case 'failed': return 'Erreur';
    default: return 'Idle';
  }
}
</script>

<template>
  <div class="service-table-container">
    <table class="service-table">
      <thead>
        <tr>
          <th>Service</th>
          <th v-if="showWorkspace" class="hide-mobile">Workspace</th>
          <th class="hide-tablet">Technologie</th>
          <th>Statut</th>
          <th class="hide-mobile">Port(s)</th>
          <th class="hide-tablet">Dernière activité</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="item in services" :key="item.workspace.id + item.config.name">
          <td>
            <div class="service-name">
              <span class="tech-icon">{{ getTechIcon(item.config.name) }}</span>
              <span>{{ item.config.display_name || item.config.name }}</span>
            </div>
          </td>
          <td v-if="showWorkspace" class="hide-mobile">{{ item.workspace.name }}</td>
          <td class="hide-tablet">
            <div class="tech-tag">
               {{ getTechIcon(item.config.name) }} {{ item.config.mode || 'Service' }}
            </div>
          </td>
          <td>
            <div class="status-cell">
              <span class="status-dot" :class="getStatusClass(item.status)"></span>
              <span class="status-label" :class="getStatusClass(item.status)">{{ getStatusLabel(item.status) }}</span>
            </div>
          </td>
          <td class="hide-mobile"><span class="port">{{ item.port || '----' }}</span></td>
          <td class="activity hide-tablet">{{ item.lastActivity || 'Il y a 2 min' }}</td>
          <td><button class="action-btn">⋮</button></td>
        </tr>
      </tbody>
    </table>
    <div class="footer">
      <a href="#" class="view-all">Voir tous les services</a>
    </div>
  </div>
</template>

<style scoped>
.service-table-container {
  background-color: #161b22;
  border: 1px solid #30363d;
  border-radius: 12px;
  overflow-x: auto;
}

.service-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.85rem;
  text-align: left;
  min-width: 800px;
}

.service-table th {
  color: #8b949e;
  font-weight: 500;
  padding: 1rem;
  border-bottom: 1px solid #30363d;
}

.service-table td {
  padding: 1rem;
  color: #f0f6fc;
  border-bottom: 1px solid #21262d;
}

.service-name {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-weight: 500;
}

.tech-tag {
  background-color: #0d1117;
  padding: 0.25rem 0.5rem;
  border-radius: 6px;
  font-size: 0.75rem;
  border: 1px solid #30363d;
  display: inline-block;
}

.status-cell {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background-color: #8b949e;
}

.status-label { color: #8b949e; }

.port {
  color: #58a6ff;
  font-family: monospace;
}

.activity {
  color: #8b949e;
}

.action-btn {
  background: none;
  border: none;
  color: #8b949e;
  cursor: pointer;
}

.footer {
  padding: 0.75rem 1rem;
}

.view-all {
  color: #58a6ff;
  text-decoration: none;
  font-size: 0.8rem;
}

.view-all:hover {
  text-decoration: underline;
}

@media (max-width: 900px) {
  .hide-tablet {
    display: none;
  }
}

@media (max-width: 600px) {
  .hide-mobile {
    display: none;
  }
  
  .service-table {
    min-width: unset;
  }

  .service-table th, .service-table td {
    padding: 0.75rem 0.5rem;
  }
}
</style>
