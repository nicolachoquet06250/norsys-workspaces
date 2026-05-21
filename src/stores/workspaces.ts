import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { computed, ref } from "vue";
import type { ServiceConfig, WorkspaceConfig } from "../types";

export const useWorkspacesStore = defineStore("workspaces", () => {
  const items = ref<WorkspaceConfig[]>([]);
  const selectedWorkspaceId = ref<string | null>(null);
  const isLoading = ref(false);
  const isCreating = ref(false);
  const isDeleting = ref(false);
  const error = ref<string | null>(null);
  let dockerEventsUnlisten: UnlistenFn | null = null;

  type DockerWorkspaceEventPayload = {
    refreshRuntime?: boolean;
    workspaces?: WorkspaceConfig[];
  };

  const selectedWorkspace = computed<WorkspaceConfig | null>(() => {
    return items.value.find((workspace) => workspace.id === selectedWorkspaceId.value) ?? null;
  });

  async function fetchWorkspaces() {
    isLoading.value = true;
    error.value = null;
    try {
      items.value = await invoke<WorkspaceConfig[]>("list_workspaces");
    } catch (fetchError) {
      error.value = fetchError instanceof Error ? fetchError.message : "Impossible de charger les workspaces";
    } finally {
      isLoading.value = false;
    }
  }

  function applyDockerWorkspacePayload(payload: DockerWorkspaceEventPayload) {
    if (!payload.refreshRuntime || !Array.isArray(payload.workspaces)) {
      return;
    }

    items.value = payload.workspaces;

    if (selectedWorkspaceId.value && !items.value.some((workspace) => workspace.id === selectedWorkspaceId.value)) {
      selectedWorkspaceId.value = items.value[0]?.id ?? null;
    }
  }

  async function initDockerEventsListener() {
    if (dockerEventsUnlisten) {
      return;
    }

    dockerEventsUnlisten = await listen("docker:event", (event) => {
      applyDockerWorkspacePayload(event.payload as DockerWorkspaceEventPayload);
    });
  }

  function disposeDockerEventsListener() {
    if (!dockerEventsUnlisten) {
      return;
    }

    dockerEventsUnlisten();
    dockerEventsUnlisten = null;
  }

  function selectWorkspace(workspaceId: string) {
    selectedWorkspaceId.value = workspaceId;
  }

  function clearSelectedWorkspace() {
    selectedWorkspaceId.value = null;
  }

  async function createWorkspace(name: string, root: string, services: ServiceConfig[] = []) {
    const trimmedName = name.trim();
    const trimmedRoot = root.trim();

    if (!trimmedName || !trimmedRoot) {
      throw new Error("Le nom et le chemin racine sont obligatoires.");
    }

    isCreating.value = true;
    error.value = null;

    try {
      const newWorkspace = await invoke<WorkspaceConfig>("create_workspace", {
        workspace: {
          id: crypto.randomUUID(),
          name: trimmedName,
          root: trimmedRoot,
          services,
          open: [],
          env_files: [],
          env: {},
        },
      });

      await fetchWorkspaces();
      selectWorkspace(newWorkspace.id);
      return newWorkspace;
    } catch (createError) {
      error.value = createError instanceof Error ? createError.message : "Impossible de créer le workspace";
      throw createError;
    } finally {
      isCreating.value = false;
    }
  }

  async function deleteWorkspace(workspaceId: string) {
    const trimmedId = workspaceId.trim();

    if (!trimmedId) {
      throw new Error("L'identifiant du workspace est requis.");
    }

    isDeleting.value = true;
    error.value = null;

    try {
      await invoke("delete_workspace", { workspaceId: trimmedId });
      await fetchWorkspaces();

      if (selectedWorkspaceId.value === trimmedId) {
        selectedWorkspaceId.value = items.value[0]?.id ?? null;
      }
    } catch (deleteError) {
      error.value = deleteError instanceof Error ? deleteError.message : "Impossible de supprimer le workspace";
      throw deleteError;
    } finally {
      isDeleting.value = false;
    }
  }

  return {
    items,
    selectedWorkspaceId,
    isLoading,
    isCreating,
    isDeleting,
    error,
    selectedWorkspace,
    fetchWorkspaces,
    initDockerEventsListener,
    disposeDockerEventsListener,
    createWorkspace,
    deleteWorkspace,
    selectWorkspace,
    clearSelectedWorkspace,
  };
});
