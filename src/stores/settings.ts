import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import {computed, onMounted, ref} from "vue";

type UiState = "idle" | "stopping" | "starting" | "running" | "error";

export const useSettingsStore = defineStore("settings", () => {
  const uiState = ref<UiState>("idle");
  const lastWorkspaceId = ref<string | null>(null);
  const username = ref<string>("Utilisateur");
  const email = ref<string>("utilisateur@example.com");
  const isDockerConnected = ref<boolean>(false);
  const accentColor = ref<string>("gray");
  const isAccentColorLight = ref<boolean>(true);

  onMounted(async () => {
    if (accentColor.value === "gray") {
      accentColor.value = (await invoke<string>('get_accent_color')).replace("#", '');
    }

    isAccentColorLight.value = await invoke('is_light_accent_color');
  })

  async function loadPersistedSettings() {
    const persisted = await invoke<{ last_workspace_id?: string | null; ui_state?: UiState }>("get_persisted_settings");
    lastWorkspaceId.value = persisted.last_workspace_id ?? null;
    uiState.value = persisted.ui_state ?? "idle";
    
    try {
      username.value = await invoke<string>("get_os_username");
    } catch (e) {
      console.error("Failed to get OS username", e);
    }

    try {
      email.value = await invoke<string>("get_os_email");
    } catch (e) {
      console.error("Failed to get OS email, falling back to generated one", e);
      email.value = `${username.value.toLowerCase().replace(/\s+/g, ".")}@example.com`;
    }

    await checkDockerConnection();
  }

  async function checkDockerConnection() {
    try {
      isDockerConnected.value = await invoke<boolean>("is_docker_connected");
    } catch (e) {
      console.error("Failed to check Docker connection", e);
      isDockerConnected.value = false;
    }
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
    username,
    email,
    isDockerConnected,
    accentColor,
    profilePictureTextColor: computed(() => isAccentColorLight.value ? '000000' : 'FFFFFF'),
    loadPersistedSettings,
    persistSettings,
    setUiState,
    setLastWorkspace,
    checkDockerConnection,
  };
});
