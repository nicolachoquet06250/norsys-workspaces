<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { useLogsStore } from "../stores/logs";
import { useWorkspacesStore } from "../stores/workspaces";

type AggregatedLog = {
  workspaceId: string;
  workspaceName: string;
  line: string;
  arrivalOrder: number;
};

const workspacesStore = useWorkspacesStore();
const logsStore = useLogsStore();
const pollIntervalMs = 1500;
const maxVisibleLogs = 2000;

const aggregatedLogs = ref<AggregatedLog[]>([]);
const logsContainer = ref<HTMLElement | null>(null);
const shouldAutoScroll = ref(true);

let poller: number | null = null;
let arrivalCounter = 0;

const workspaceNameById = computed<Record<string, string>>(() => {
  return Object.fromEntries(workspacesStore.items.map((workspace) => [workspace.id, workspace.name]));
});

const knownLengthsByWorkspaceId = ref<Record<string, number>>({});

const hasLogs = computed(() => aggregatedLogs.value.length > 0);

function pushAggregatedLog(workspaceId: string, line: string) {
  arrivalCounter += 1;
  aggregatedLogs.value.push({
    workspaceId,
    workspaceName: workspaceNameById.value[workspaceId] ?? workspaceId,
    line,
    arrivalOrder: arrivalCounter,
  });

  if (aggregatedLogs.value.length > maxVisibleLogs) {
    aggregatedLogs.value.splice(0, aggregatedLogs.value.length - maxVisibleLogs);
  }
}

async function pullWorkspaceLogs(workspaceId: string) {
  await logsStore.fetchLogs(workspaceId);
  const workspaceLogs = logsStore.byWorkspaceId[workspaceId] ?? [];
  const previousLength = knownLengthsByWorkspaceId.value[workspaceId] ?? 0;

  if (workspaceLogs.length > previousLength) {
    for (const line of workspaceLogs.slice(previousLength)) {
      pushAggregatedLog(workspaceId, line);
    }
  }

  knownLengthsByWorkspaceId.value[workspaceId] = workspaceLogs.length;
}

async function refreshAllLogs() {
  if (workspacesStore.items.length === 0) {
    await workspacesStore.fetchWorkspaces();
  }

  for (const workspace of workspacesStore.items) {
    await pullWorkspaceLogs(workspace.id);
  }
}

function clearAllLogs() {
  aggregatedLogs.value = [];
  knownLengthsByWorkspaceId.value = {};
  arrivalCounter = 0;
  for (const workspace of workspacesStore.items) {
    logsStore.clearWorkspaceLogs(workspace.id);
  }
}

function onLogScroll(event: Event) {
  const target = event.target as HTMLElement;
  const threshold = 24;
  shouldAutoScroll.value = target.scrollHeight - target.scrollTop - target.clientHeight < threshold;
}

watch(
  () => aggregatedLogs.value.length,
  async () => {
    if (!shouldAutoScroll.value) {
      return;
    }

    await nextTick();
    if (logsContainer.value) {
      logsContainer.value.scrollTop = logsContainer.value.scrollHeight;
    }
  },
);

onMounted(async () => {
  await refreshAllLogs();
  poller = window.setInterval(() => {
    void refreshAllLogs();
  }, pollIntervalMs);
});

onUnmounted(() => {
  if (poller !== null) {
    clearInterval(poller);
    poller = null;
  }
});
</script>

<template>
  <div class="logs-view">
    <header class="header">
      <div>
        <h1>Logs</h1>
        <p class="subtitle">Flux temps réel de tous les workspaces, dans l'ordre d'arrivée.</p>
      </div>
      <button class="btn-clear" @click="clearAllLogs">Vider</button>
    </header>

    <section class="content">
      <p v-if="workspacesStore.isLoading" class="state">Chargement des workspaces...</p>
      <p v-else-if="workspacesStore.error" class="state state-error">{{ workspacesStore.error }}</p>
      <p v-else-if="logsStore.error" class="state state-error">{{ logsStore.error }}</p>

      <div v-else-if="hasLogs" ref="logsContainer" class="logs-container" @scroll="onLogScroll">
        <article v-for="log in aggregatedLogs" :key="`${log.arrivalOrder}-${log.workspaceId}`" class="log-line">
          <span class="workspace">[{{ log.workspaceName }}]</span>
          <span class="message">{{ log.line }}</span>
        </article>
      </div>

      <div v-else class="empty-state">
        <p>Aucun log reçu pour le moment.</p>
      </div>
    </section>
  </div>
</template>

<style scoped>
.logs-view {
  min-height: 100vh;
  padding: 2rem;
  background: #0d1117;
  color: #f0f6fc;
}

.header {
  margin-bottom: 1.5rem;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

h1 {
  font-size: 1.8rem;
}

.subtitle {
  color: #8b949e;
  margin-top: 0.35rem;
}

.content {
  border: 1px solid #30363d;
  border-radius: 10px;
  background: #161b22;
  min-height: 70vh;
  overflow: hidden;
}

.state,
.empty-state {
  padding: 1rem;
}

.state-error {
  color: #f85149;
}

.logs-container {
  height: 70vh;
  overflow: auto;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, "Liberation Mono", monospace;
  font-size: 0.82rem;
  line-height: 1.4;
  padding: 0.75rem;
}

.log-line {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 0.65rem;
  padding: 0.2rem 0;
  border-bottom: 1px solid rgba(139, 148, 158, 0.15);
}

.workspace {
  color: #79c0ff;
  white-space: nowrap;
}

.message {
  color: #c9d1d9;
  word-break: break-word;
}

.btn-clear {
  border: 1px solid #30363d;
  border-radius: 8px;
  padding: 0.4rem 0.75rem;
  background: #161b22;
  color: #f0f6fc;
  cursor: pointer;
}

.btn-clear:hover {
  border-color: #58a6ff;
}
</style>