<script setup lang="ts">
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { ref } from "vue";
import { useRouter } from "vue-router";
import { useSettingsStore } from "../stores/settings";
import { useWorkspacesStore } from "../stores/workspaces";
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
  <main class="create-container">
    <header class="panel hero">
      <div style="display: flex; flex-direction: row; justify-content: flex-start; align-items: center; gap: 0.5rem;">
        <button
            class="back-link"
            @click="router.push({ name: 'home' })"
            aria-label="Retour à l'accueil"
            title="Retour à l'accueil"
            :disabled="workspacesStore.isCreating"
        >
          🏠
        </button>
        <p class="eyebrow">Nouveau workspace</p>
      </div>
      <h1>Créer un workspace</h1>
      <p class="subtitle">Renseignez les informations minimales pour ajouter un espace de travail.</p>
    </header>

    <section class="panel">
      <form class="create-form" @submit.prevent="createWorkspace">
        <fieldset :disabled="workspacesStore.isCreating" style="border: none; padding: 0; margin: 0; display: grid; gap: 0.9rem;">
          <label>
            Nom du workspace
            <input v-model="newWorkspaceName" type="text" placeholder="Ex: Projet API" />
          </label>
          <label>
            Chemin racine
            <div class="path-selector" @click="!workspacesStore.isCreating && selectWorkspaceDirectory()" tabindex="0" @keydown.enter.space.prevent="!workspacesStore.isCreating && selectWorkspaceDirectory()" :class="{ disabled: workspacesStore.isCreating }">
              <button
                type="button"
                class="pick-folder-button"
                aria-label="Sélectionner un dossier"
                title="Sélectionner un dossier"
                tabindex="-1"
              >
                📁
              </button>
              <span v-if="newWorkspaceRoot" class="selected-path-inline">{{ newWorkspaceRoot }}</span>
              <span v-else class="path-placeholder">Aucun dossier sélectionné</span>
            </div>
          </label>
          <div v-if="newWorkspaceRoot" class="detected-services">
            <p class="detected-services-title">Services Docker détectés :</p>
            <ul v-if="detectedServices.length > 0" class="detected-services-list">
              <li v-for="service in detectedServices" :key="service.name">{{ service.name }}</li>
            </ul>
            <p v-else class="detected-services-empty">Aucun service détecté dans `docker-compose.yaml`.</p>
          </div>
          <button
            class="primary"
            type="submit"
            :disabled="workspacesStore.isCreating"
            aria-label="Créer le workspace"
            title="Créer le workspace"
          >
            {{ workspacesStore.isCreating ? "⏳ Création..." : "✅ Créer le workspace" }}
          </button>
        </fieldset>
      </form>
      <p v-if="createError" class="error">{{ createError }}</p>
    </section>
  </main>
</template>

<style scoped>
.create-container {
  max-width: 760px;
  margin: 0 auto;
  padding: 2rem 1.25rem 2.5rem;
  color: #14213d;
}

.panel {
  border: 1px solid #d9e2f1;
  border-radius: 14px;
  background: #fff;
  box-shadow: 0 10px 24px rgba(17, 24, 39, 0.06);
  padding: 1.2rem;
}

.hero {
  background: linear-gradient(135deg, #f7faff, #eef4ff);
  margin-bottom: 1rem;
}

.eyebrow {
  margin: 0;
  font-size: 0.78rem;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: #4f5d75;
}

h1 {
  margin: 0.2rem 0 0;
}

.subtitle {
  margin: 0.45rem 0 0;
  color: #4f5d75;
}

.back-link {
  border: none;
  background: transparent;
  color: #396cd8;
  cursor: pointer;
  padding: 0;
}

.back-link:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.path-selector.disabled {
  background: #f1f5f9;
  cursor: not-allowed;
  opacity: 0.7;
}

.create-form {
  display: grid;
  gap: 0.9rem;
}

.create-form label {
  display: grid;
  gap: 0.35rem;
  font-weight: 600;
  color: #1f2a44;
}

.create-form input {
  border: 1px solid #cbd5e1;
  border-radius: 10px;
  padding: 0.65rem 0.8rem;
  font-size: 0.95rem;
}

.pick-folder-button {
  border: 1px solid #cbd5e1;
  border-radius: 10px;
  background: #fff;
  padding: 0.65rem 0.8rem;
  font-size: 0.95rem;
  color: #1f2a44;
  cursor: pointer;
  transition: border-color 0.15s ease, box-shadow 0.15s ease, background-color 0.15s ease;
  flex-shrink: 0;
}

.path-selector {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  border: 1px solid #cbd5e1;
  border-radius: 10px;
  padding: 0.25rem;
  background: #fff;
  cursor: pointer;
  transition: border-color 0.15s ease, box-shadow 0.15s ease;
  width: 100%;
  box-sizing: border-box;
  overflow: hidden;
}

.path-selector:hover {
  border-color: #396cd8;
}

.path-selector:focus-within,
.path-selector:focus-visible {
  outline: none;
  border-color: #396cd8;
  box-shadow: 0 0 0 3px rgba(57, 108, 216, 0.2);
}

.path-selector .pick-folder-button {
  border: none;
  background: none;
  pointer-events: none;
}

.selected-path-inline,
.path-placeholder {
  font-size: 0.9rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  flex-grow: 1;
  min-width: 0;
  display: block;
  padding-right: 0.5rem;
}

.selected-path-inline {
  color: #1f2a44;
}

.path-placeholder {
  color: #94a3b8;
  font-style: italic;
}

.pick-folder-button:hover {
  border-color: #396cd8;
  background: #f7faff;
}

.pick-folder-button:focus-visible {
  outline: none;
  border-color: #396cd8;
  box-shadow: 0 0 0 3px rgba(57, 108, 216, 0.2);
}


.detected-services {
  border: 1px solid #dce7fa;
  border-radius: 10px;
  padding: 0.65rem 0.8rem;
  background: #f8fbff;
}

.detected-services-title {
  margin: 0;
  font-weight: 600;
}

.detected-services-list {
  margin: 0.4rem 0 0;
  padding-left: 1.2rem;
}

.detected-services-empty {
  margin: 0.4rem 0 0;
  color: #4f5d75;
}

button.primary {
  border: 1px solid #396cd8;
  border-radius: 10px;
  padding: 0.65rem 1rem;
  background: #396cd8;
  color: #fff;
  font-weight: 600;
  cursor: pointer;
}

button.primary:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.error {
  color: #b42318;
}
</style>