<script setup lang="ts">
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { ref } from "vue";
import { useRouter } from "vue-router";
import { useSettingsStore } from "../stores/settings";
import { useWorkspacesStore } from "../stores/workspaces";
import RightPanel from "../components/dashboard/RightPanel.vue";
import type { ServiceConfig } from "../types";

const router = useRouter();
const workspacesStore = useWorkspacesStore();
const settingsStore = useSettingsStore();

const newWorkspaceName = ref("");
const newWorkspaceRoot = ref("");
const detectedServices = ref<ServiceConfig[]>([]);
const createError = ref<string | null>(null);

async function selectWorkspaceDirectory() {
  createError.value = null;

  const selectedPath = await open({
    directory: true,
    multiple: false,
    title: "Sélectionner le dossier racine du workspace",
  });

  if (!selectedPath) {
    return;
  }

  if (Array.isArray(selectedPath)) {
    newWorkspaceRoot.value = selectedPath[0] ?? "";
  } else {
    newWorkspaceRoot.value = selectedPath;
  }

  if (newWorkspaceRoot.value && !newWorkspaceName.value.trim()) {
    const path = newWorkspaceRoot.value;
    const parts = path.split(/[\\/]/).filter(Boolean);
    if (parts.length > 0) {
      newWorkspaceName.value = parts[parts.length - 1];
    }
  }

  if (!newWorkspaceRoot.value) {
    detectedServices.value = [];
    return;
  }

  try {
    detectedServices.value = await invoke<ServiceConfig[]>("detect_docker_services", {
      root: newWorkspaceRoot.value,
    });
  } catch (detectError) {
    detectedServices.value = [];
    createError.value = detectError instanceof Error ? detectError.message : "Impossible de détecter les services Docker";
  }
}

async function createWorkspace() {
  createError.value = null;

  if (!newWorkspaceName.value.trim() || !newWorkspaceRoot.value.trim()) {
    createError.value = "Le nom et le chemin racine sont obligatoires.";
    return;
  }

  try {
    const createdWorkspace = await workspacesStore.createWorkspace(
      newWorkspaceName.value,
      newWorkspaceRoot.value,
      detectedServices.value,
    );
    settingsStore.setLastWorkspace(createdWorkspace.id);
    router.push({ name: "workspace-detail", params: { id: createdWorkspace.id } });
  } catch {
    createError.value = workspacesStore.error ?? "Impossible de créer le workspace";
  }
}
</script>

<template>
  <div class="dashboard">
    <div class="main-content">
      <header class="dashboard-header">
        <div class="welcome">
          <div class="breadcrumb">
            <span class="clickable" @click="router.push('/')">Accueil</span>
            <span class="separator">/</span>
            <span class="current">Nouveau workspace</span>
          </div>
          <h1>Créer un workspace</h1>
          <p class="subtitle">Renseignez les informations minimales pour ajouter un espace de travail.</p>
        </div>
        <div class="header-actions">
          <button
              class="btn-outline"
              @click="router.push({ name: 'home' })"
              :disabled="workspacesStore.isCreating"
          >
            Annuler
          </button>
        </div>
      </header>

      <section class="section">
        <div class="create-card">
          <form class="create-form" @submit.prevent="createWorkspace">
            <fieldset :disabled="workspacesStore.isCreating">
              <div class="form-group">
                <label for="ws-name">Nom du workspace</label>
                <input id="ws-name" v-model="newWorkspaceName" type="text" placeholder="Ex: Projet API" />
              </div>

              <div class="form-group">
                <label>Chemin racine</label>
                <div class="path-selector" @click="!workspacesStore.isCreating && selectWorkspaceDirectory()" tabindex="0" @keydown.enter.space.prevent="!workspacesStore.isCreating && selectWorkspaceDirectory()" :class="{ disabled: workspacesStore.isCreating }">
                  <span class="folder-icon">📁</span>
                  <span v-if="newWorkspaceRoot" class="selected-path-inline">{{ newWorkspaceRoot }}</span>
                  <span v-else class="path-placeholder">Aucun dossier sélectionné</span>
                  <button
                    type="button"
                    class="browse-btn"
                    tabindex="-1"
                  >
                    Parcourir
                  </button>
                </div>
              </div>

              <div v-if="newWorkspaceRoot" class="detected-services">
                <p class="detected-services-title">Services Docker détectés :</p>
                <div v-if="detectedServices.length > 0" class="tech-tags">
                  <span v-for="service in detectedServices" :key="service.name" class="tech-tag">
                    📦 {{ service.name }}
                  </span>
                </div>
                <p v-else class="detected-services-empty">Aucun service détecté dans `docker-compose.yaml`.</p>
              </div>

              <div class="form-actions">
                <button
                  class="btn-primary"
                  type="submit"
                  :disabled="workspacesStore.isCreating"
                >
                  <span v-if="workspacesStore.isCreating">⏳ Création...</span>
                  <span v-else>Créer le workspace</span>
                </button>
              </div>
            </fieldset>
          </form>
          <p v-if="createError" class="error-msg">{{ createError }}</p>
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

.breadcrumb {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.85rem;
  color: #8b949e;
  margin-bottom: 0.5rem;
}

.breadcrumb .clickable {
  cursor: pointer;
}

.breadcrumb .clickable:hover {
  color: #58a6ff;
  text-decoration: underline;
}

.breadcrumb .separator {
  color: #484f58;
}

.breadcrumb .current {
  color: #f0f6fc;
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
  gap: 1rem;
}

.btn-outline {
  background-color: #161b22;
  border: 1px solid #30363d;
  color: #f0f6fc;
  padding: 0.6rem 1.2rem;
  border-radius: 8px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-outline:hover:not(:disabled) {
  background-color: #21262d;
  border-color: #8b949e;
}

.btn-outline:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.section {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.create-card {
  background-color: #161b22;
  border: 1px solid #30363d;
  border-radius: 12px;
  padding: 2rem;
}

.create-form fieldset {
  border: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.form-group label {
  color: #f0f6fc;
  font-weight: 500;
  font-size: 0.9rem;
}

.form-group input {
  background-color: #0d1117;
  border: 1px solid #30363d;
  border-radius: 6px;
  padding: 0.6rem 0.8rem;
  color: #f0f6fc;
  font-size: 0.95rem;
  transition: border-color 0.2s;
}

.form-group input:focus {
  outline: none;
  border-color: #1f6feb;
  box-shadow: 0 0 0 3px rgba(31, 111, 235, 0.1);
}

.path-selector {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  background-color: #0d1117;
  border: 1px solid #30363d;
  border-radius: 6px;
  padding: 0.5rem 0.8rem;
  cursor: pointer;
  transition: border-color 0.2s;
}

.path-selector:hover:not(.disabled) {
  border-color: #8b949e;
}

.path-selector.disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.folder-icon {
  font-size: 1.1rem;
}

.selected-path-inline {
  color: #f0f6fc;
  font-size: 0.9rem;
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.path-placeholder {
  color: #484f58;
  font-size: 0.9rem;
  font-style: italic;
  flex: 1;
}

.browse-btn {
  background-color: #21262d;
  border: 1px solid #30363d;
  color: #c9d1d9;
  padding: 0.3rem 0.6rem;
  border-radius: 4px;
  font-size: 0.8rem;
  cursor: pointer;
}

.detected-services {
  background-color: #0d1117;
  border: 1px solid #30363d;
  border-radius: 8px;
  padding: 1rem;
}

.detected-services-title {
  color: #8b949e;
  font-size: 0.85rem;
  margin-bottom: 0.75rem;
  font-weight: 500;
}

.tech-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.tech-tag {
  background-color: #161b22;
  border: 1px solid #30363d;
  color: #f0f6fc;
  padding: 0.25rem 0.6rem;
  border-radius: 6px;
  font-size: 0.8rem;
}

.detected-services-empty {
  color: #484f58;
  font-size: 0.85rem;
  font-style: italic;
}

.form-actions {
  margin-top: 1rem;
  display: flex;
  justify-content: flex-end;
}

.btn-primary {
  background-color: #1f6feb;
  color: white;
  border: 1px solid rgba(240, 246, 252, 0.1);
  padding: 0.6rem 1.5rem;
  border-radius: 8px;
  font-weight: 600;
  cursor: pointer;
  transition: background-color 0.2s;
}

.btn-primary:hover:not(:disabled) {
  background-color: #388bfd;
}

.btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.error-msg {
  color: #f85149;
  font-size: 0.85rem;
  margin-top: 1rem;
  padding: 0.75rem;
  background-color: rgba(248, 81, 73, 0.1);
  border: 1px solid rgba(248, 81, 73, 0.2);
  border-radius: 6px;
}

@media (max-width: 1000px) {
  .dashboard {
    flex-direction: column;
  }
}
</style>