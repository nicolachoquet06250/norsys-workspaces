<script setup lang="ts">
import type { WorkspaceConfig } from "../../types";

defineProps<{
  workspace: WorkspaceConfig;
  status: string;
}>();

defineEmits<{
  (e: 'click', id: string): void;
}>();

function getTechIcon(name: string) {
  const n = name.toLowerCase();
  if (n.includes('api')) return '🟢 JS';
  if (n.includes('web')) return '🔵 React';
  if (n.includes('backend')) return '🟣 .NET';
  if (n.includes('data') || n.includes('pipeline')) return '🟠 Python';
  return '📦';
}

function getStatusLabel(status: string) {
  switch (status) {
    case 'running': return 'En cours';
    case 'starting': return 'Démarrage...';
    case 'stopping': return 'Arrêt...';
    case 'stopped': return 'Arrêté';
    case 'failed': return 'Erreur';
    case 'blocked': return 'Bloqué';
    default: return 'En attente';
  }
}
</script>

<template>
  <div class="workspace-card" @click="$emit('click', workspace.id)">
    <div class="card-header">
      <div class="tech-icon">{{ getTechIcon(workspace.name) }}</div>
      <div class="info">
        <span class="name">{{ workspace.name }}</span>
        <span class="path">{{ workspace.root }}</span>
      </div>
      <button class="options-btn">⋮</button>
    </div>
    
    <div class="status-row">
      <span class="status-dot" :class="status"></span>
      <span class="status-text" :class="status">{{ getStatusLabel(status) }}</span>
    </div>
    
    <div class="card-footer">
      <div class="footer-item">
        <span class="icon">🧩</span>
        <span>{{ workspace.services.length }} services</span>
      </div>
      <div class="footer-item">
        <span class="icon">🏷️</span>
        <span>{{ getTechIcon(workspace.name).split(' ')[1] }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.workspace-card {
  background-color: #161b22;
  border: 1px solid #30363d;
  border-radius: 12px;
  padding: 1rem;
  cursor: pointer;
  transition: all 0.2s;
}

.workspace-card:hover {
  border-color: #58a6ff;
  background-color: #1c2128;
}

.card-header {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  margin-bottom: 1rem;
}

.tech-icon {
  background-color: rgba(255, 255, 255, 0.05);
  padding: 0.5rem;
  border-radius: 8px;
  font-size: 0.8rem;
  white-space: nowrap;
}

.info {
  display: flex;
  flex-direction: column;
  flex-grow: 1;
  overflow: hidden;
}

.name {
  color: #f0f6fc;
  font-weight: 600;
  font-size: 0.95rem;
}

.path {
  color: #8b949e;
  font-size: 0.75rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.options-btn {
  background: none;
  border: none;
  color: #8b949e;
  cursor: pointer;
  font-size: 1.2rem;
  padding: 0 0.25rem;
}

.status-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 1rem;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background-color: #8b949e;
}

.status-text {
  font-size: 0.85rem;
  color: #8b949e;
}

.card-footer {
  display: flex;
  gap: 1rem;
  padding-top: 0.75rem;
  border-top: 1px solid #30363d;
}

.footer-item {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  color: #8b949e;
  font-size: 0.8rem;
}
</style>
