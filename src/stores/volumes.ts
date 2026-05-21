import {defineStore} from "pinia";
import {listen, type UnlistenFn} from "@tauri-apps/api/event";
import {computed, ref} from "vue";
import type {WorkspaceServiceVolume} from "../types";

type DockerVolumesEventPayload = {
  type?: string;
  action?: string;
  refreshVolumes?: boolean;
  volume?: WorkspaceServiceVolume;
  volumes?: WorkspaceServiceVolume[];
};

export const useVolumesStore = defineStore("volumes", () => {
  const items = ref<WorkspaceServiceVolume[]>([]);
  const isLoading = ref(true);
  const error = ref<string | null>(null);
  const byKey = ref<Record<string, WorkspaceServiceVolume>>({});
  let dockerEventsUnlisten: UnlistenFn | null = null;

  const hasVolumes = computed(() => items.value.length > 0);

  function applyDockerVolumesPayload(payload: DockerVolumesEventPayload) {
    if (payload.action === "sync") {
      isLoading.value = false;
    }

    if (payload.type === "volume" && payload.action === "mount") {
      const mountedVolumes = Array.isArray(payload.volumes)
        ? payload.volumes
        : payload.volume
          ? [payload.volume]
          : [];

      if (mountedVolumes.length === 0) {
        isLoading.value = false;
        return;
      }

      const nextByKey = { ...byKey.value };

      for (const volume of mountedVolumes) {
        const key = `${volume.workspace_id}::${volume.service_name}::${volume.volume}`;
        nextByKey[key] = volume;
      }

      byKey.value = nextByKey;
      items.value = Object.values(nextByKey);
      error.value = null;
      isLoading.value = false;
      return;
    }

    if (payload.action !== "start" || !payload.refreshVolumes || !Array.isArray(payload.volumes)) {
      return;
    }

    const nextByKey = { ...byKey.value };

    for (const volume of payload.volumes) {
      const key = `${volume.workspace_id}::${volume.service_name}::${volume.volume}`;
      nextByKey[key] = volume;
    }

    byKey.value = nextByKey;
    items.value = Object.values(nextByKey);
    error.value = null;
    isLoading.value = false;
  }

  async function initDockerEventsListener() {
    if (dockerEventsUnlisten) {
      isLoading.value = false;
      return;
    }

    dockerEventsUnlisten = await listen("docker:event", (event) => {
      applyDockerVolumesPayload(event.payload as DockerVolumesEventPayload);
    });

    isLoading.value = false;
  }

  function disposeDockerEventsListener() {
    if (!dockerEventsUnlisten) {
      return;
    }

    dockerEventsUnlisten();
    dockerEventsUnlisten = null;
  }

  return {
    items,
    isLoading,
    error,
    hasVolumes,
    initDockerEventsListener,
    disposeDockerEventsListener,
  };
});
