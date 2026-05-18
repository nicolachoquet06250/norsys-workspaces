import {defineStore} from "pinia";
import {invoke} from "@tauri-apps/api/core";
import {ref} from "vue";
import type {RuntimeWorkspaceState, WorkspaceStartResponse} from "../types";

export const useRuntimeStore = defineStore("runtime", () => {
  const byWorkspaceId = ref<Record<string, RuntimeWorkspaceState>>({});
  const isStarting = ref(false);
  const isStopping = ref(false);
  const error = ref<string | null>(null);

  function setWorkspaceStarting(workspaceId: string, serviceNames: string[]) {
    byWorkspaceId.value[workspaceId] = {
      workspace_id: workspaceId,
      global_status: "starting",
      services: serviceNames.map((serviceName) => ({
        name: serviceName,
        status: "starting",
      })),
    };
  }

  function setWorkspaceStopping(workspaceId: string) {
    const current = byWorkspaceId.value[workspaceId];
    if (!current) {
      return;
    }

    byWorkspaceId.value[workspaceId] = {
      ...current,
      global_status: "stopping",
      services: current.services.map((service) => ({
        ...service,
        status: "stopping",
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
    } catch (startError) {
      error.value = startError instanceof Error ? startError.message : `Impossible de démarrer le workspace : ${startError}`;
    } finally {
      isStarting.value = false;
    }
  }

  async function refreshWorkspaceState(workspaceId: string) {
    byWorkspaceId.value[workspaceId] = await invoke<RuntimeWorkspaceState>("get_workspace_runtime_state", {workspaceId});
  }

  async function stopWorkspace(workspaceId: string) {
    isStopping.value = true;
    error.value = null;
    setWorkspaceStopping(workspaceId);
    try {
      byWorkspaceId.value[workspaceId] = await invoke<RuntimeWorkspaceState>("stop_workspace", {workspaceId});
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

  return {
    byWorkspaceId,
    isStarting,
    isStopping,
    error,
    setWorkspaceStarting,
    setWorkspaceStopping,
    startWorkspace,
    refreshWorkspaceState,
    stopWorkspace,
    stopWorkspaceProbes,
  };
});
