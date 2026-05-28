<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useLogsStore } from "../stores/logs";
import { useRuntimeStore } from "../stores/runtime";
import { useSettingsStore } from "../stores/settings";
import { useNotificationsStore } from "../stores/notifications";
import { useWorkspacesStore } from "../stores/workspaces";
import StatCard from "../components/dashboard/StatCard.vue";
import ServiceTable from "../components/dashboard/ServiceTable.vue";
import RightPanel from "../components/dashboard/RightPanel.vue";
import WorkspaceDocs from "../components/workspace/WorkspaceDocs.vue";
import type { RuntimeWorkspaceState, WorkspaceEnvFile } from "../types";

const route = useRoute();
const router = useRouter();
const workspacesStore = useWorkspacesStore();
const runtimeStore = useRuntimeStore();
const logsStore = useLogsStore();
const settingsStore = useSettingsStore();
const notificationsStore = useNotificationsStore();
const runtimePollingDelayMs = 1000;
const isDebugMode = import.meta.env.DEV;

let runtimePollingHandle: number | null = null;
let isRuntimeRefreshInFlight = false;

const activeTab = ref<"services" | "logs" | "env" | "docs">("services");
const envFiles = ref<WorkspaceEnvFile[]>([]);
const mergedEnv = ref<Record<string, string>>({});
const isLoadingEnv = ref(false);
const activeEnvFileIndex = ref(-1);

async function loadEnvData() {
  if (!workspacesStore.selectedWorkspaceId) return;
  isLoadingEnv.value = true;
  try {
    const [files, env] = await Promise.all([
      workspacesStore.getWorkspaceEnvFiles(workspacesStore.selectedWorkspaceId),
      workspacesStore.getWorkspaceMergedEnv(workspacesStore.selectedWorkspaceId)
    ]);
    envFiles.value = files;
    mergedEnv.value = env;
  } catch (e) {
    console.error("Failed to load environment data", e);
  } finally {
    isLoadingEnv.value = false;
  }
}

watch(activeTab, (newTab) => {
  if (newTab === "env") {
    loadEnvData();
  }
});

const sortedMergedEnvKeys = computed(() => {
  return Object.keys(mergedEnv.value).sort();
});

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
      display_name: service.display_name,
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

const activeServicesCount = computed(() => {
  if (!selectedRuntime.value) return 0;
  return selectedRuntime.value.services.filter(s => s.status === 'running').length;
});

function normalizeServiceName(name: string): string {
  return name.trim().toLowerCase().replace(/[-_\s]+/g, "");
}

function markNotificationsAsRead() {
  notificationsStore.markAsRead();
}

function normalizeDisplayedPort(port: string): string {
  const trimmedPort = port.trim();
  const [leftPort] = trimmedPort.split(":");
  return leftPort?.trim() || trimmedPort;
}

function isWebService(service: { name: string; mode?: string }): boolean {
  const mode = (service.mode ?? "").toLowerCase();
  const name = service.name.toLowerCase();
  return (
    mode.includes("web") ||
    mode.includes("front") ||
    mode.includes("back") ||
    name.includes("nginx") ||
    name.includes("apache") ||
    name.includes("caddy")
  );
}

function buildAccessUrl(port: string | undefined): string | undefined {
  if (!port) {
    return undefined;
  }

  const normalizedPort = normalizeDisplayedPort(port);
  if (!/^\d+$/.test(normalizedPort)) {
    return undefined;
  }

  const protocol = normalizedPort === "443" ? "https" : "http";
  return `${protocol}://localhost:${normalizedPort}`;
}

const livePortsByService = computed(() => {
  const workspaceId = workspacesStore.selectedWorkspaceId;
  if (!workspaceId) {
    return new Map<string, string[]>();
  }

  const liveWorkspace = workspacesStore.items.find((workspace) => workspace.id === workspaceId);
  const portsByService = new Map<string, string[]>();

  for (const service of liveWorkspace?.services ?? []) {
    const ports =
      service.ports
        ?.map((port) => normalizeDisplayedPort(port))
        .filter((port) => port.length > 0) ?? [];
    portsByService.set(normalizeServiceName(service.name), ports);
  }

  return portsByService;
});

/*const getRawServiceStatus = (serviceName: string) => {
  const runtimeService = selectedRuntimeOrDefault.value?.services.find((service) => service.name === serviceName);
  return runtimeService?.status ?? "idle";
};*/

const serviceTableData = computed(() => {
  if (!selectedWorkspace.value) return [];
  
  return selectedWorkspace.value.services.map(service => {
    const normalizedServiceName = normalizeServiceName(service.name);
    const sState = selectedRuntime.value?.services.find((runtimeService) => {
      return normalizeServiceName(runtimeService.name) === normalizedServiceName;
    });
    const servicePorts = livePortsByService.value.get(normalizedServiceName) ?? [];

    return {
      config: service,
      workspace: selectedWorkspace.value!,
      status: sState?.status || 'idle',
      port: servicePorts.length > 0 ? servicePorts.join(", ") : undefined,
      accessUrl: isWebService(service) ? buildAccessUrl(servicePorts[0]) : undefined,
      lastActivity: sState?.status === 'running' ? 'Actif' : 'Inactif'
    };
  });
});

onMounted(async () => {
  await workspacesStore.initDockerEventsListener();
  await runtimeStore.initDockerEventsListener();
  await settingsStore.loadPersistedSettings();
  await workspacesStore.fetchWorkspaces();
  applyRouteSelection();
});

watch(routeWorkspaceId, applyRouteSelection);

watch(
  () => workspacesStore.selectedWorkspaceId,
  async (workspaceId) => {
    if (!workspaceId) {
      stopRuntimePolling();
      return;
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
  runtimeStore.disposeDockerEventsListener();
  workspacesStore.disposeDockerEventsListener();

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
    await workspacesStore.fetchWorkspaces();
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
  
  if (route.name === 'workspace-docs') {
    activeTab.value = 'docs';
  }

  if (workspacesStore.selectedWorkspaceId === routeWorkspaceId.value) {
    return;
  }
  workspacesStore.selectWorkspace(routeWorkspaceId.value);
}

/*function selectWorkspace(workspaceId: string) {
  workspacesStore.selectWorkspace(workspaceId);
  settingsStore.setLastWorkspace(workspaceId);
  router.push({ name: "workspace-detail", params: { id: workspaceId } });
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
}*/

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
  <div class="dashboard">
    <div class="main-content">
      <template v-if="selectedWorkspace">
        <header class="dashboard-header">
          <div class="welcome">
            <div class="breadcrumb">
              <span @click="router.push('/')" class="clickable">Accueil</span>
              <span class="separator">/</span>
              <span class="current">Workspace</span>
            </div>
            <h1>{{ selectedWorkspace.name }} 📁</h1>
            <p class="subtitle">{{ selectedWorkspace.root }}</p>
          </div>
          <div class="header-actions">
            <div class="action-buttons">
              <button 
                class="btn-action start" 
                @click="startSelectedWorkspace"
                :disabled="runtimeStore.isStarting"
              >
                <span class="icon">{{ runtimeStore.isStarting ? '⏳' : '▶️' }}</span> Démarrer
              </button>
              <button 
                class="btn-action stop" 
                @click="stopSelectedWorkspace"
                :disabled="runtimeStore.isStopping"
              >
                <span class="icon">{{ runtimeStore.isStopping ? '⏳' : '⏹️' }}</span> Arrêter
              </button>
              <button 
                class="btn-action delete" 
                @click="deleteSelectedWorkspace"
                :disabled="workspacesStore.isDeleting"
              >
                <span class="icon">{{ workspacesStore.isDeleting ? '⏳' : '🗑️' }}</span> Supprimer
              </button>
            </div>
            <div class="header-icons">
              <button class="icon-btn">🔍</button>
              <button class="icon-btn badge" @click="markNotificationsAsRead">🔔<span class="count">{{ notificationsStore.count }}</span></button>
              <button class="icon-btn">⚙️</button>
            </div>
          </div>
        </header>

        <div class="stats-row">
          <StatCard 
            title="Services" 
            :value="selectedWorkspace.services.length" 
            :subValue="activeServicesCount + ' actifs'" 
            icon="🧩" 
            color="#3fb950" 
          />
          <StatCard 
            title="Statut Global" 
            :value="selectedRuntimeOrDefault?.global_status || 'idle'" 
            :subValue="'Mode: ' + (selectedWorkspace.services[0]?.mode || 'N/A')" 
            icon="🚥" 
            color="#58a6ff" 
          />
          <StatCard 
            title="Logs" 
            :value="selectedLogs.length" 
            subValue="Lignes capturées" 
            icon="📋" 
            color="#d29922" 
          />
        </div>

        <div class="tabs">
          <button 
            class="tab-btn" 
            :class="{ active: activeTab === 'services' }" 
            @click="activeTab = 'services'"
          >
            🧩 Services
          </button>
          <button 
            class="tab-btn" 
            :class="{ active: activeTab === 'logs' }" 
            @click="activeTab = 'logs'"
          >
            📋 Logs
          </button>
          <button 
            class="tab-btn" 
            :class="{ active: activeTab === 'env' }" 
            @click="activeTab = 'env'"
          >
            🔑 Environnement
          </button>
          <button 
            class="tab-btn" 
            :class="{ active: activeTab === 'docs' }" 
            @click="activeTab = 'docs'"
          >
            📚 Documentation
          </button>
        </div>

        <section v-if="activeTab === 'services'" class="section">
          <div class="section-header">
            <h2>Services du workspace</h2>
            <div class="filters">
               <select><option>Trier par nom</option></select>
            </div>
          </div>
          <ServiceTable :services="serviceTableData" :show-workspace="false" :show-access-url-column="true" />
        </section>

        <section v-if="activeTab === 'logs'" class="section">
          <div class="section-header">
             <h2>Journaux d'exécution</h2>
             <button class="btn-outline" @click="logsStore.clearWorkspaceLogs(selectedWorkspace.id)">Effacer</button>
          </div>
          <div class="logs-container">
            <div v-if="logsStore.error" class="error-msg">{{ logsStore.error }}</div>
            <div v-else-if="selectedLogs.length === 0" class="empty-logs">
              <p>Aucun log disponible pour le moment.</p>
            </div>
            <ul v-else class="logs-list">
              <li v-for="(log, index) in selectedLogs" :key="index">{{ log }}</li>
            </ul>
          </div>
        </section>

        <section v-if="activeTab === 'env'" class="section">
          <div class="section-header">
            <h2>Variables d'environnement</h2>
            <button class="btn-outline" @click="loadEnvData" :disabled="isLoadingEnv">
              {{ isLoadingEnv ? 'Chargement...' : 'Actualiser' }}
            </button>
          </div>

          <div v-if="isLoadingEnv" class="env-loading">Chargement des données d'environnement...</div>
          <div v-else class="env-content">
            <div class="env-files-sidebar">
              <h3>Fichiers détectés</h3>
              <ul class="env-files-list">
                <li 
                  v-for="(file, index) in envFiles" 
                  :key="file.name"
                  :class="{ active: activeEnvFileIndex === index }"
                  @click="activeEnvFileIndex = index"
                >
                  {{ file.name }}
                </li>
                <li 
                  :class="{ active: activeEnvFileIndex === -1 }"
                  @click="activeEnvFileIndex = -1"
                >
                  Variables fusionnées (résultat)
                </li>
              </ul>
            </div>

            <div class="env-viewer">
              <div v-if="activeEnvFileIndex === -1" class="merged-env-view">
                <table class="env-table">
                  <thead>
                    <tr>
                      <th>Variable</th>
                      <th>Valeur</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="key in sortedMergedEnvKeys" :key="key">
                      <td class="env-key">{{ key }}</td>
                      <td class="env-value">{{ mergedEnv[key] }}</td>
                    </tr>
                  </tbody>
                </table>
              </div>
              <div v-else-if="envFiles[activeEnvFileIndex]" class="file-content-view">
                <pre class="env-file-content">{{ envFiles[activeEnvFileIndex].content }}</pre>
              </div>
              <div v-else class="empty-env">
                Aucun fichier .env détecté à la racine.
              </div>
            </div>
          </div>
        </section>

        <section v-if="activeTab === 'docs'" class="section">
          <WorkspaceDocs :workspaceId="selectedWorkspace.id" />
        </section>
      </template>

      <div v-else class="empty-state">
        <div class="empty-icon">📂</div>
        <h2>Workspace introuvable</h2>
        <p>Veuillez sélectionner un workspace valide ou retourner à l'accueil.</p>
        <button class="btn-primary" @click="router.push('/')">Retour à l'accueil</button>
      </div>
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
  word-break: break-all;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 1.5rem;
  flex-wrap: wrap;
}

.action-buttons {
  display: flex;
  gap: 0.75rem;
}

.btn-action {
  border: 1px solid #30363d;
  background-color: #161b22;
  color: #f0f6fc;
  padding: 0.6rem 1rem;
  border-radius: 8px;
  font-weight: 600;
  display: flex;
  align-items: center;
  gap: 0.5rem;
  cursor: pointer;
  transition: all 0.2s;
  font-size: 0.9rem;
}

.btn-action:hover:not(:disabled) {
  background-color: #21262d;
  border-color: #8b949e;
}

.btn-action.start:hover:not(:disabled) {
  border-color: #3fb950;
  color: #3fb950;
}

.btn-action.stop:hover:not(:disabled) {
  border-color: #d29922;
  color: #d29922;
}

.btn-action.delete:hover:not(:disabled) {
  border-color: #f85149;
  color: #f85149;
}

.btn-action:disabled {
  opacity: 0.5;
  cursor: not-allowed;
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

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1.25rem;
  gap: 1rem;
  flex-wrap: wrap;
}

.section h2 {
  font-size: 1.1rem;
  color: #f0f6fc;
  font-weight: 600;
}

.tabs {
  display: flex;
  gap: 0.5rem;
  border-bottom: 1px solid #30363d;
  padding-bottom: 0.5rem;
  margin-top: 1rem;
}

.tab-btn {
  background: transparent;
  border: none;
  padding: 0.5rem 1rem;
  color: #8b949e;
  cursor: pointer;
  font-size: 0.95rem;
  font-weight: 500;
  border-radius: 6px;
  transition: all 0.2s;
}

.tab-btn:hover {
  background: #161b22;
  color: #f0f6fc;
}

.tab-btn.active {
  color: #58a6ff;
  background: #161b22;
  position: relative;
}

.tab-btn.active::after {
  content: "";
  position: absolute;
  bottom: -0.6rem;
  left: 0;
  right: 0;
  height: 2px;
  background: #58a6ff;
}

.env-content {
  display: flex;
  gap: 1.5rem;
  min-height: 400px;
}

.env-files-sidebar {
  width: 250px;
  border-right: 1px solid #30363d;
  padding-right: 1rem;
}

.env-files-sidebar h3 {
  font-size: 0.9rem;
  color: #8b949e;
  margin-bottom: 1rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.env-files-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.env-files-list li {
  padding: 0.5rem 0.75rem;
  border-radius: 6px;
  cursor: pointer;
  font-size: 0.9rem;
  color: #f0f6fc;
  transition: background 0.2s;
}

.env-files-list li:hover {
  background: #21262d;
}

.env-files-list li.active {
  background: #1f6feb;
  color: white;
}

.env-viewer {
  flex: 1;
  background: #0d1117;
  border: 1px solid #30363d;
  border-radius: 8px;
  overflow: auto;
  max-height: 600px;
}

.env-file-content {
  padding: 1rem;
  margin: 0;
  font-family: 'Cascadia Code', 'Fira Code', monospace;
  font-size: 0.9rem;
  white-space: pre-wrap;
  color: #d1d5da;
}

.env-table {
  width: 100%;
  border-collapse: collapse;
}

.env-table th {
  text-align: left;
  padding: 0.75rem 1rem;
  border-bottom: 1px solid #30363d;
  color: #8b949e;
  font-size: 0.85rem;
  font-weight: 600;
  background: #161b22;
}

.env-table td {
  padding: 0.75rem 1rem;
  border-bottom: 1px solid #21262d;
  font-size: 0.9rem;
}

.env-key {
  color: #79c0ff;
  font-family: monospace;
  width: 30%;
}

.env-value {
  color: #a5d6ff;
  font-family: monospace;
  word-break: break-all;
}

.empty-env, .env-loading {
  padding: 2rem;
  text-align: center;
  color: #8b949e;
}

.filters select {
  background-color: #161b22;
  border: 1px solid #30363d;
  color: #8b949e;
  padding: 0.4rem 0.8rem;
  border-radius: 6px;
  font-size: 0.85rem;
}

.btn-outline {
  background: none;
  border: 1px solid #30363d;
  color: #8b949e;
  padding: 0.4rem 0.8rem;
  border-radius: 6px;
  font-size: 0.85rem;
  cursor: pointer;
}

.btn-outline:hover {
  border-color: #8b949e;
  color: #f0f6fc;
}

.logs-container {
  background-color: #161b22;
  border: 1px solid #30363d;
  border-radius: 12px;
  padding: 1rem;
}

.logs-list {
  list-style: none;
  padding: 0;
  margin: 0;
  max-height: 400px;
  overflow-y: auto;
  font-family: 'Cascadia Code', 'Fira Code', monospace;
  font-size: 0.85rem;
}

.logs-list li {
  padding: 0.25rem 0.5rem;
  border-bottom: 1px solid #21262d;
  color: #d1d5da;
  word-break: break-all;
}

.logs-list li:last-child {
  border-bottom: none;
}

.empty-logs, .error-msg {
  padding: 2rem;
  text-align: center;
}

.empty-logs p {
  color: #8b949e;
  font-size: 0.9rem;
  margin: 0;
}

.error-msg {
  color: #f85149;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 5rem 2rem;
  text-align: center;
  background-color: #161b22;
  border: 1px dashed #30363d;
  border-radius: 16px;
}

.empty-icon {
  font-size: 4rem;
  margin-bottom: 1.5rem;
}

.empty-state h2 {
  color: #f0f6fc;
  margin-bottom: 1rem;
}

.empty-state p {
  color: #8b949e;
  margin-bottom: 2rem;
}

.btn-primary {
  background-color: #1f6feb;
  color: white;
  border: none;
  padding: 0.75rem 1.5rem;
  border-radius: 8px;
  font-weight: 600;
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
    flex-direction: column;
    align-items: stretch;
  }

  .action-buttons {
    justify-content: space-between;
  }
}
</style>
