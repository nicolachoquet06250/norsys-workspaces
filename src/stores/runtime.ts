import {defineStore} from "pinia";
import {invoke} from "@tauri-apps/api/core";
import {ref} from "vue";
import type {RecentRun, RuntimeWorkspaceState, SystemStats, WorkspaceStartResponse} from "../types";

export const useRuntimeStore = defineStore("runtime", () => {
  const byWorkspaceId = ref<Record<string, RuntimeWorkspaceState>>({});
  const recentRuns = ref<RecentRun[]>([]);
  const isStarting = ref(false);
  const isStopping = ref(false);
  const error = ref<string | null>(null);
  const systemStats = ref<SystemStats | null>(null);

  function setWorkspaceStarting(workspaceId: string, serviceNames: string[]) {
    const now = Date.now();
    byWorkspaceId.value[workspaceId] = {
      workspace_id: workspaceId,
      global_status: "starting",
      services: serviceNames.map((serviceName) => ({
        name: serviceName,
        status: "starting",
        last_transition: now,
      })),
    };
  }

  function setWorkspaceStopping(workspaceId: string) {
    const current = byWorkspaceId.value[workspaceId];
    if (!current) {
      return;
    }

    const now = Date.now();
    byWorkspaceId.value[workspaceId] = {
      ...current,
      global_status: "stopping",
      services: current.services.map((service) => ({
        ...service,
        status: "stopping",
        last_transition: service.status !== "stopping" ? now : service.last_transition,
      })),
    };
  }

  async function startWorkspace(workspaceId: string, serviceNames: string[] = []) {
    isStarting.value = true;
    error.value = null;
    setWorkspaceStarting(workspaceId, serviceNames);
    try {
      await invoke<WorkspaceStartResponse>("start_workspace", { workspaceId });
      await refreshWorkspaceState(workspaceId);
      await loadRecentRuns();
    } catch (startError) {
      error.value = startError instanceof Error ? startError.message : `Impossible de démarrer le workspace : ${startError}`;
    } finally {
      isStarting.value = false;
    }
  }

  async function refreshWorkspaceState(workspaceId: string) {
    const newState = await invoke<RuntimeWorkspaceState>("get_workspace_runtime_state", {workspaceId});
    const oldState = byWorkspaceId.value[workspaceId];
    const now = Date.now();

    if (oldState) {
      for (const newService of newState.services) {
        const oldService = oldState.services.find(s => s.name === newService.name);
        if (oldService && oldService.status !== newService.status) {
          newService.last_transition = now;
          // Persister le changement d'état
          await invoke("add_recent_run", {
            workspaceId,
            serviceName: newService.name,
            action: `status_change_to_${newService.status}`,
            status: "success"
          });
          await loadRecentRuns();
        } else {
          newService.last_transition = oldService?.last_transition;
        }
      }
    } else {
      newState.services = newState.services.map(s => ({ ...s, last_transition: now }));
    }

    byWorkspaceId.value[workspaceId] = newState;
  }

  async function stopWorkspace(workspaceId: string) {
    isStopping.value = true;
    error.value = null;
    setWorkspaceStopping(workspaceId);
    try {
      const newState = await invoke<RuntimeWorkspaceState>("stop_workspace", {workspaceId});
      const oldState = byWorkspaceId.value[workspaceId];
      const now = Date.now();

      if (oldState) {
        for (const newService of newState.services) {
          const oldService = oldState.services.find(s => s.name === newService.name);
          if (oldService && oldService.status !== newService.status) {
            newService.last_transition = now;
          } else {
            newService.last_transition = oldService?.last_transition;
          }
        }
      } else {
        newState.services = newState.services.map(s => ({ ...s, last_transition: now }));
      }
      byWorkspaceId.value[workspaceId] = newState;
      await loadRecentRuns();
    } catch (stopError) {
      error.value = stopError instanceof Error ? stopError.message : `Impossible d'arrêter le workspace: ${stopError}`;
    } finally {
      isStopping.value = false;
    }
  }

  async function stopWorkspaceProbes(workspaceId: string) {
    const trimmedId = workspaceId.trim();
    if (!trimmedId) {
      return;
    }

    await invoke("stop_workspace_probes", { workspaceId: trimmedId });
  }

  async function loadRecentRuns() {
    try {
      recentRuns.value = await invoke<RecentRun[]>("get_recent_runs", { limit: 10 });
    } catch (e) {
      console.error("Failed to load recent runs", e);
    }
  }

  async function updateSystemStats() {
    try {
      systemStats.value = await invoke<SystemStats>("get_system_stats");
    } catch (e) {
      console.error("Failed to update system stats", e);
    }
  }

  return {
    byWorkspaceId,
    recentRuns,
    isStarting,
    isStopping,
    error,
    systemStats,
    setWorkspaceStarting,
    setWorkspaceStopping,
    startWorkspace,
    refreshWorkspaceState,
    stopWorkspace,
    stopWorkspaceProbes,
    loadRecentRuns,
    updateSystemStats,
  };
});
