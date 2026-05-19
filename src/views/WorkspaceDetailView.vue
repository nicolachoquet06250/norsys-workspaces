<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import StatusIndicator from "../components/StatusIndicator.vue";
import { useLogsStore } from "../stores/logs";
import { useRuntimeStore } from "../stores/runtime";
import { useSettingsStore } from "../stores/settings";
import { useWorkspacesStore } from "../stores/workspaces";
import type { RuntimeWorkspaceState } from "../types";

const route = useRoute();
const router = useRouter();
const workspacesStore = useWorkspacesStore();
const runtimeStore = useRuntimeStore();
const logsStore = useLogsStore();
const settingsStore = useSettingsStore();
const runtimePollingDelayMs = 1000;
const isDebugMode = import.meta.env.DEV;

let runtimePollingHandle: number | null = null;
let isRuntimeRefreshInFlight = false;

const routeWorkspaceId = computed(() => String(route.params.id ?? ""));

function formatLogWithCreationDate(logLine: string): string {
  const timestamp = logsStore.extractTimestamp(logLine);
  if (!timestamp) {
    return `[DATE_INCONNUE] ${logLine}`;
  }
  return `[${timestamp}] ${logLine}`;
}

const selectedWorkspace = computed(() => workspacesStore.selectedWorkspace);
const selectedRuntime = computed(() => {
  if (!workspacesStore.selectedWorkspaceId) {
    return null;
  }
  return runtimeStore.byWorkspaceId[workspacesStore.selectedWorkspaceId] ?? null;
});
const selectedRuntimeOrDefault = computed<RuntimeWorkspaceState | null>(() => {
  if (!selectedWorkspace.value) {
    return null;
  }

  if (selectedRuntime.value) {
    return selectedRuntime.value;
  }

  return {
    workspace_id: selectedWorkspace.value.id,
    global_status: "idle",
    services: selectedWorkspace.value.services.map((service) => ({
      name: service.name,
      status: "idle",
    })),
  };
});
const selectedLogs = computed(() => {
  if (!workspacesStore.selectedWorkspaceId) {
    return [];
  }
  const rawLogs = logsStore.byWorkspaceId[workspacesStore.selectedWorkspaceId] ?? [];
  return rawLogs.map(formatLogWithCreationDate);
});

onMounted(async () => {
  await settingsStore.loadPersistedSettings();
  await workspacesStore.fetchWorkspaces();
  applyRouteSelection();
});

watch(routeWorkspaceId, applyRouteSelection);

watch(
  () => workspacesStore.selectedWorkspaceId,
  async (workspaceId, previousWorkspaceId) => {
    if (!workspaceId) {
      stopRuntimePolling();
      return;
    }

    if (previousWorkspaceId && previousWorkspaceId !== workspaceId) {
      await runtimeStore.stopWorkspaceProbes(previousWorkspaceId);
    }

    try {
      await runtimeStore.refreshWorkspaceState(workspaceId);
    } catch {
      stopRuntimePolling();
      return;
    }

    const runtime = runtimeStore.byWorkspaceId[workspaceId];
    if (!runtime) {
      stopRuntimePolling();
      return;
    }

    if (runtime.global_status === "running" || runtime.global_status === "starting") {
      startRuntimePolling(workspaceId, { skipImmediateRefresh: true });
      return;
    }

    stopRuntimePolling();
  },
  { immediate: true },
);

onUnmounted(() => {
  if (workspacesStore.selectedWorkspaceId) {
    void runtimeStore.stopWorkspaceProbes(workspacesStore.selectedWorkspaceId);
  }
  stopRuntimePolling();
});

async function refreshRuntimeAndSync(workspaceId: string) {
  if (isRuntimeRefreshInFlight) {
    return;
  }

  if (workspacesStore.selectedWorkspaceId !== workspaceId) {
    stopRuntimePolling();
    return;
  }

  isRuntimeRefreshInFlight = true;
  try {
    await runtimeStore.refreshWorkspaceState(workspaceId);
    void logsStore.fetchLogs(workspaceId);
    syncUiStateFromRuntime(workspaceId);
  } catch {
    settingsStore.setUiState("error");
  } finally {
    isRuntimeRefreshInFlight = false;
  }
}

function startRuntimePolling(workspaceId: string, options: { skipImmediateRefresh?: boolean } = {}) {
  stopRuntimePolling();
  runtimePollingHandle = window.setInterval(() => {
    void refreshRuntimeAndSync(workspaceId);
  }, runtimePollingDelayMs);
  if (!options.skipImmediateRefresh) {
    void refreshRuntimeAndSync(workspaceId);
  }
}

function stopRuntimePolling() {
  if (runtimePollingHandle !== null) {
    window.clearInterval(runtimePollingHandle);
    runtimePollingHandle = null;
  }
}

function applyRouteSelection() {
  if (!routeWorkspaceId.value) {
    return;
  }
  if (workspacesStore.selectedWorkspaceId === routeWorkspaceId.value) {
    return;
  }
  workspacesStore.selectWorkspace(routeWorkspaceId.value);
}

function selectWorkspace(workspaceId: string) {
  workspacesStore.selectWorkspace(workspaceId);
  settingsStore.setLastWorkspace(workspaceId);
  router.push({ name: "workspace-detail", params: { id: workspaceId } });
}

function getRawServiceStatus(serviceName: string): any {
  const runtimeService = selectedRuntimeOrDefault.value?.services.find((service) => service.name === serviceName);
  return runtimeService?.status ?? "idle";
}

async function copyServiceCommand(command: string) {
  const shouldPrefixWithWsl = navigator.userAgent.toLowerCase().includes("windows");
  const commandToCopy = shouldPrefixWithWsl ? `wsl ${command}` : command;

  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(commandToCopy);
    return;
  }

  const textarea = document.createElement("textarea");
  textarea.value = commandToCopy;
  textarea.setAttribute("readonly", "");
  textarea.style.position = "absolute";
  textarea.style.left = "-9999px";
  document.body.appendChild(textarea);
  textarea.select();
  document.execCommand("copy");
  document.body.removeChild(textarea);
}

function syncUiStateFromRuntime(workspaceId: string): "starting" | "running" | "error" {
  const runtime = runtimeStore.byWorkspaceId[workspaceId];
  if (!runtime) {
    settingsStore.setUiState("error");
    return "error";
  }

  if (runtime.global_status === "failed") {
    settingsStore.setUiState("error");
    return "error";
  }

  if (runtime.global_status === "running") {
    settingsStore.setUiState("running");
    return "running";
  }

  settingsStore.setUiState("starting");
  return "starting";
}

async function waitUntilWorkspaceReady(workspaceId: string) {
  const maxAttempts = 20;
  const delayMs = 1000;

  for (let attempt = 0; attempt < maxAttempts; attempt += 1) {
    await runtimeStore.refreshWorkspaceState(workspaceId);
    const next = syncUiStateFromRuntime(workspaceId);
    if (next !== "starting") {
      return;
    }
    await new Promise((resolve) => window.setTimeout(resolve, delayMs));
  }
}

async function startSelectedWorkspace() {
  if (!workspacesStore.selectedWorkspaceId) {
    return;
  }
  const workspaceId = workspacesStore.selectedWorkspaceId;

  settingsStore.setUiState("starting");
  logsStore.clearWorkspaceLogs(workspaceId);
  const serviceNames = selectedWorkspace.value?.services.map((service) => service.name) ?? [];
  await runtimeStore.startWorkspace(workspaceId, serviceNames);
  await logsStore.fetchLogs(workspaceId);
  settingsStore.setLastWorkspace(workspaceId);

  if (runtimeStore.error) {
    if (isDebugMode) {
      logsStore.appendDebugLog(workspaceId, `Échec du démarrage du workspace: ${runtimeStore.error}`);
    }
    settingsStore.setUiState("error");
    return;
  }

  syncUiStateFromRuntime(workspaceId);
  startRuntimePolling(workspaceId);
  await waitUntilWorkspaceReady(workspaceId);
}

async function stopSelectedWorkspace() {
  if (!workspacesStore.selectedWorkspaceId) {
    return;
  }
  const workspaceId = workspacesStore.selectedWorkspaceId;

  settingsStore.setUiState("stopping");
  runtimeStore.setWorkspaceStopping(workspaceId);
  await nextTick();

  stopRuntimePolling();
  await runtimeStore.stopWorkspace(workspaceId);
  settingsStore.setUiState("idle");
}

async function deleteSelectedWorkspace() {
  const workspaceToDelete = selectedWorkspace.value;
  if (!workspaceToDelete) {
    return;
  }

  const confirmed = window.confirm(`Supprimer le workspace \"${workspaceToDelete.name}\" ?`);
  if (!confirmed) {
    return;
  }

  try {
    await workspacesStore.deleteWorkspace(workspaceToDelete.id);

    if (settingsStore.lastWorkspaceId === workspaceToDelete.id) {
      settingsStore.setLastWorkspace(null);
    }

    const nextWorkspaceId = workspacesStore.selectedWorkspaceId;
    if (nextWorkspaceId) {
      router.push({ name: "workspace-detail", params: { id: nextWorkspaceId } });
      return;
    }

    router.push({ name: "home" });
  } catch {
    // message d'erreur déjà géré par le store
  }
}
</script>

<template>
  <main class="container">
    <header class="page-header">
      <div style="display: flex; flex-direction: row; justify-content: flex-start; align-items: center; gap: 0.5rem;">
        <button
            class="back-link"
            @click="router.push({ name: 'home' })"
            aria-label="Retour à l'accueil"
            title="Retour à l'accueil"
            :disabled="workspacesStore.isDeleting || runtimeStore.isStarting || runtimeStore.isStopping"
        >
          🏠
        </button>
        <p class="eyebrow">Détail d'un workspace</p>
      </div>
      <h1>Dev Workspace Manager</h1>
      <p>Tableau de bord opérationnel de votre environnement</p>
    </header>

    <section class="dashboard-layout">
      <aside class="panel sidebar-panel">
        <h2>Espaces de travail</h2>
        <p v-if="workspacesStore.isLoading">Chargement...</p>
        <p v-else-if="workspacesStore.error" class="error">{{ workspacesStore.error }}</p>
        <ul v-else class="workspace-list">
          <li v-for="workspace in workspacesStore.items" :key="workspace.id">
            <button
              class="workspace-item"
              :class="{ active: workspacesStore.selectedWorkspaceId === workspace.id }"
              @click="selectWorkspace(workspace.id)"
              :disabled="workspacesStore.isDeleting || runtimeStore.isStarting || runtimeStore.isStopping"
            >
              {{ workspace.name }}
            </button>
          </li>
        </ul>
      </aside>

      <section class="content-panel">
        <template v-if="selectedWorkspace">
          <div class="top-panels">
            <article class="panel">
              <p class="panel-kicker">Vue d'ensemble</p>
              <h2>{{ selectedWorkspace.name }}</h2>
              <p class="root-path"><strong>Chemin racine :</strong> <span>{{ selectedWorkspace.root }}</span></p>
              <div style="display: flex; align-items: center; gap: 0.5rem;">
                <strong>État global :</strong>
                <StatusIndicator v-if="selectedRuntimeOrDefault" :status="selectedRuntimeOrDefault.global_status" show-text />
              </div>
            </article>

            <article class="panel">
              <h3>Actions</h3>
              <div class="actions">
                <button
                  @click="startSelectedWorkspace"
                  :disabled="runtimeStore.isStarting"
                  aria-label="Démarrer les services"
                  title="Démarrer les services"
                >
                  {{ runtimeStore.isStarting ? "⏳" : "▶️" }}
                </button>
                <button
                  @click="stopSelectedWorkspace"
                  :disabled="runtimeStore.isStopping"
                  aria-label="Arrêter le workspace"
                  title="Arrêter le workspace"
                >
                  {{ runtimeStore.isStopping ? "⏳" : "⏹️" }}
                </button>
                <button
                  class="danger"
                  @click="deleteSelectedWorkspace"
                  :disabled="workspacesStore.isDeleting"
                  aria-label="Supprimer le workspace"
                  title="Supprimer le workspace"
                >
                  {{ workspacesStore.isDeleting ? "⏳" : "🗑️" }}
                </button>
              </div>
            </article>
          </div>

          <article class="panel">
            <h3>Services</h3>
            <div class="services-table-wrapper">
              <table class="services-table">
                <thead>
                  <tr>
                    <th>Nom du service</th>
                    <th>Commande</th>
                    <th>État</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="service in selectedWorkspace.services" :key="service.name">
                    <td class="service-name">{{ service.name }}</td>
                    <td>
                      <div class="command-cell">
                        <code class="service-command">
                          <span class="service-command-text">{{ service.command }}</span>
                          <button
                            class="copy-command-button"
                            type="button"
                            aria-label="Copier la commande"
                            title="Copier la commande"
                            @click="copyServiceCommand(service.command)"
                          >
                            📋
                          </button>
                        </code>
                      </div>
                    </td>
                    <td>
                      <StatusIndicator :status="getRawServiceStatus(service.name)" show-text />
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </article>

          <article class="panel">
            <h3>Journaux</h3>
            <p v-if="logsStore.error" class="error">{{ logsStore.error }}</p>
            <ul v-else class="logs-list">
              <li v-for="(log, index) in selectedLogs" :key="index">{{ log }}</li>
            </ul>
          </article>
        </template>

        <article v-else class="panel empty-state">
          <h2>Workspace introuvable</h2>
          <p>Sélectionnez un workspace dans la colonne de gauche pour afficher son tableau de bord.</p>
        </article>
      </section>
    </section>

    <p v-if="runtimeStore.error" class="error">{{ runtimeStore.error }}</p>
  </main>
</template>

<style scoped>
.container {
  max-width: 1320px;
  margin: 0 auto;
  padding: 2rem 1.25rem;
  color: #14213d;
}

.page-header {
  margin-bottom: 1.5rem;
}

.back-link {
  border: none;
  background: transparent;
  color: #396cd8;
  font-weight: 700;
  padding: 0;
  cursor: pointer;
}

.eyebrow {
  margin: 0 0 0.35rem;
  font-size: 0.78rem;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: #4f5d75;
}

.page-header h1 {
  margin: 0;
  font-size: 1.9rem;
}

.page-header p {
  margin: 0.35rem 0 0;
  color: #4f5d75;
}

.dashboard-layout {
  display: grid;
  grid-template-columns: 320px minmax(0, 1fr);
  gap: 1.25rem;
  align-items: start;
}

.panel {
  border: 1px solid #d9e2f1;
  border-radius: 12px;
  padding: 1rem 1.1rem;
  margin-bottom: 1.1rem;
  background: #ffffff;
  box-shadow: 0 8px 20px rgba(17, 24, 39, 0.05);
}

.panel h2,
.panel h3 {
  margin-top: 0;
}

.panel-kicker {
  margin: 0 0 0.5rem;
  font-size: 0.8rem;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: #396cd8;
}

.sidebar-panel {
  position: sticky;
  top: 1rem;
}

.workspace-list {
  list-style: none;
  padding: 0;
  margin: 0;
}

.back-link:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.workspace-item {
  width: 100%;
  text-align: left;
  margin-bottom: 0.55rem;
  border: 1px solid #d9e2f1;
  border-radius: 10px;
  padding: 0.65rem 0.75rem;
  background: #f8fbff;
  cursor: pointer;
  transition: all 0.2s ease;
}

.workspace-item:hover {
  border-color: #99b7f3;
  background: #eef4ff;
}

.content-panel {
  min-width: 0;
}

.top-panels {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 1.25rem;
}

.top-panels .panel {
  margin-bottom: 0;
}

.workspace-item.active {
  border-color: #396cd8;
  background: #eaf1ff;
  font-weight: 600;
}

.services-table-wrapper {
  overflow-x: auto;
}

.services-table {
  width: 100%;
  border-collapse: collapse;
}

.services-table th,
.services-table td {
  border-bottom: 1px solid #d9e2f1;
  padding: 0.65rem 0.5rem;
  text-align: left;
  vertical-align: top;
}

.services-table th {
  font-size: 0.82rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: #4f5d75;
}

.service-name {
  font-weight: 700;
}

.command-cell {
  display: block;
}

.service-command {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  max-width: 100%;
  background: #f5f7fb;
  border: 1px solid #d9e2f1;
  border-radius: 8px;
  padding: 0.25rem 0.3rem 0.25rem 0.55rem;
  font-family: "Consolas", "Courier New", monospace;
  font-size: 0.82rem;
  color: #14213d;
}

.service-command-text {
  overflow-x: auto;
  white-space: nowrap;
  scrollbar-width: thin;
}

.copy-command-button {
  border: 1px solid #c3d4f8;
  border-radius: 6px;
  background: #ffffff;
  color: #396cd8;
  padding: 0.2rem 0.45rem;
  line-height: 1;
  font-size: 0.78rem;
  font-weight: 500;
  cursor: pointer;
  flex-shrink: 0;
}

.copy-command-button:hover {
  background: #eef4ff;
}

.root-path {
  word-break: break-all;
  overflow-wrap: break-word;
}

.root-path span {
  color: #4f5d75;
}

.actions {
  display: flex;
  gap: 0.5rem;
  margin: 0.75rem 0 0;
}

.actions button {
  border: 1px solid #396cd8;
  border-radius: 10px;
  background: #fff;
  color: #396cd8;
  padding: 0.55rem 0.8rem;
  font-weight: 600;
  cursor: pointer;
}

.actions button:hover {
  background: #eef4ff;
}

.actions button:last-child {
  background: #fff;
  color: #396cd8;
}

.actions button.danger {
  background: #fff;
  color: #c40000;
  border-color: #c40000;
}

.actions button.danger:hover {
  background: #fff3f3;
}

.actions button:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.logs-list {
  list-style: none;
  padding: 0;
  margin: 0;
  max-height: 280px;
  overflow: auto;
}

.logs-list li {
  font-family: "Consolas", "Courier New", monospace;
  font-size: 0.84rem;
  padding: 0.35rem 0.5rem;
  border-radius: 6px;
  background: #f5f7fb;
  margin-bottom: 0.35rem;
}

.error {
  color: #c40000;
  font-weight: 600;
  white-space: pre;
}

.empty-state {
  text-align: center;
}

@media (max-width: 900px) {
  .dashboard-layout {
    grid-template-columns: 1fr;
  }

  .top-panels {
    grid-template-columns: 1fr;
  }

  .service-command {
    max-width: min(100%, 520px);
  }
}
</style>
