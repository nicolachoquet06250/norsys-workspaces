import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { ref } from "vue";

type UiState = "idle" | "stopping" | "starting" | "running" | "error";

export const useSettingsStore = defineStore("settings", () => {
  const uiState = ref<UiState>("idle");
  const lastWorkspaceId = ref<string | null>(null);

  async function loadPersistedSettings() {
    const persisted = await invoke<{ last_workspace_id?: string | null; ui_state?: UiState }>("get_persisted_settings");
    lastWorkspaceId.value = persisted.last_workspace_id ?? null;
    uiState.value = persisted.ui_state ?? "idle";
  }

  async function persistSettings() {
    await invoke("save_persisted_settings", {
      settings: {
        last_workspace_id: lastWorkspaceId.value,
        ui_state: uiState.value,
      },
    });
  }

  function setUiState(nextState: UiState) {
    uiState.value = nextState;
    void persistSettings();
  }

  function setLastWorkspace(workspaceId: string | null) {
    lastWorkspaceId.value = workspaceId;
    void persistSettings();
  }

  return {
    uiState,
    lastWorkspaceId,
    loadPersistedSettings,
    persistSettings,
    setUiState,
    setLastWorkspace,
  };
});
