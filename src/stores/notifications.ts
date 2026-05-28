import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { ref } from "vue";

type NotificationsUpdatePayload = {
  count?: number;
};

export const useNotificationsStore = defineStore("notifications", () => {
  const count = ref(0);
  let notificationsUnlisten: UnlistenFn | null = null;

  function applyPayload(payload: NotificationsUpdatePayload) {
    if (typeof payload.count !== "number") {
      return;
    }

    count.value = Math.max(0, payload.count);
  }

  async function loadNotificationsCount() {
    try {
      count.value = await invoke<number>("get_unread_notifications_count");
    } catch (error) {
      console.error("Failed to load notifications count", error);
    }
  }

  async function markAsRead() {
    try {
      await invoke("mark_notifications_as_read");
      count.value = 0;
    } catch (error) {
      console.error("Failed to mark notifications as read", error);
    }
  }

  async function initNotificationsListener() {
    if (notificationsUnlisten) {
      return;
    }

    notificationsUnlisten = await listen("notifications:update", (event) => {
      applyPayload(event.payload as NotificationsUpdatePayload);
    });
  }

  function disposeNotificationsListener() {
    if (!notificationsUnlisten) {
      return;
    }

    notificationsUnlisten();
    notificationsUnlisten = null;
  }

  return {
    count,
    loadNotificationsCount,
    markAsRead,
    initNotificationsListener,
    disposeNotificationsListener,
  };
});