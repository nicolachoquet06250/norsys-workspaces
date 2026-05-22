import {defineStore} from "pinia";
import {listen, type UnlistenFn} from "@tauri-apps/api/event";
import {computed, ref} from "vue";
import type {WorkspaceNetwork} from "../types";

type DockerNetworksEventPayload = {
  type?: string;
  action?: string;
  refreshNetworks?: boolean;
  network?: WorkspaceNetwork;
  networks?: WorkspaceNetwork[];
};

export const useNetworksStore = defineStore("networks", () => {
  const items = ref<WorkspaceNetwork[]>([]);
  const isLoading = ref(true);
  const error = ref<string | null>(null);
  const byKey = ref<Record<string, WorkspaceNetwork>>({});
  let dockerEventsUnlisten: UnlistenFn | null = null;

  const hasNetworks = computed(() => items.value.length > 0);

  function applyDockerNetworksPayload(payload: DockerNetworksEventPayload) {
    if (payload.action === "sync") {
      isLoading.value = false;
    }

    if (payload.type === "network" && payload.action === "connect") {
      const connectedNetworks = Array.isArray(payload.networks)
        ? payload.networks
        : payload.network
          ? [payload.network]
          : [];

      if (connectedNetworks.length === 0) {
        isLoading.value = false;
        return;
      }

      const nextByKey = { ...byKey.value };

      for (const network of connectedNetworks) {
        const key = `${network.workspace_id}::${network.service_name}::${network.network}`;
        nextByKey[key] = network;
      }

      byKey.value = nextByKey;
      items.value = Object.values(nextByKey);
      error.value = null;
      isLoading.value = false;
      return;
    }

    if (!payload.refreshNetworks || !Array.isArray(payload.networks)) {
      return;
    }

    const nextByKey = payload.action === 'sync' ? {} : { ...byKey.value };

    for (const network of payload.networks) {
      const key = `${network.workspace_id}::${network.service_name}::${network.network}`;
      nextByKey[key] = network;
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
      applyDockerNetworksPayload(event.payload as DockerNetworksEventPayload);
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
    hasNetworks,
    initDockerEventsListener,
    disposeDockerEventsListener,
  };
});
