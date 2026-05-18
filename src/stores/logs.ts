import {defineStore} from "pinia";
import {invoke} from "@tauri-apps/api/core";
import {ref} from "vue";

export const useLogsStore = defineStore("logs", () => {
  const byWorkspaceId = ref<Record<string, string[]>>({});
  const isLoading = ref(false);
  const error = ref<string | null>(null);
  const inFlightByWorkspaceId = ref<Record<string, boolean>>({});
  const seenTimestampsByWorkspaceId = ref<Record<string, Record<string, boolean>>>({});

  function extractTimestamp(logLine: string): string | null {
    const bracketedIsoMatch = logLine.match(/\[(\d{4}-\d{2}-\d{2}T[^\]]+)\]/);
    if (bracketedIsoMatch?.[1]) {
      return bracketedIsoMatch[1];
    }

    const plainDateTimeMatch = logLine.match(
      /\b(\d{4}-\d{2}-\d{2}[ T]\d{2}:\d{2}:\d{2}(?:[.,]\d+)?(?:Z|[+-]\d{2}:\d{2})?)\b/,
    );
    if (plainDateTimeMatch?.[1]) {
      return plainDateTimeMatch[1];
    }

    return null;
  }

  function appendDebugLog(workspaceId: string, message: string) {
    const timestamp = new Date().toISOString();
    const previousLogs = byWorkspaceId.value[workspaceId] ?? [];
    byWorkspaceId.value[workspaceId] = [...previousLogs, `[DEBUG][${timestamp}] ${message}`];
  }

  function clearWorkspaceLogs(workspaceId: string) {
    delete byWorkspaceId.value[workspaceId];
    delete seenTimestampsByWorkspaceId.value[workspaceId];
    inFlightByWorkspaceId.value[workspaceId] = false;
  }

  async function fetchLogs(workspaceId: string) {
    if (inFlightByWorkspaceId.value[workspaceId]) {
      return;
    }

    inFlightByWorkspaceId.value[workspaceId] = true;
    isLoading.value = true;
    error.value = null;
    try {
      const fetchedLogs = await invoke<string[]>("get_logs", {workspaceId});
      const previousLogs = byWorkspaceId.value[workspaceId] ?? [];

      if (fetchedLogs.length === 0 && previousLogs.length > 0) {
        return;
      }

      const seenTimestamps = seenTimestampsByWorkspaceId.value[workspaceId] ?? {};

      for (const previousLog of previousLogs) {
        const timestamp = extractTimestamp(previousLog);
        if (timestamp) {
          seenTimestamps[timestamp] = true;
        }
      }

      const uniqueLogsToAppend: string[] = [];
      for (const fetchedLog of fetchedLogs) {
        const timestamp = extractTimestamp(fetchedLog);
        if (!timestamp) {
          uniqueLogsToAppend.push(fetchedLog);
          continue;
        }

        if (seenTimestamps[timestamp]) {
          continue;
        }

        seenTimestamps[timestamp] = true;
        uniqueLogsToAppend.push(fetchedLog);
      }

      seenTimestampsByWorkspaceId.value[workspaceId] = seenTimestamps;

      if (previousLogs.length === 0) {
        byWorkspaceId.value[workspaceId] = uniqueLogsToAppend;
        return;
      }

      byWorkspaceId.value[workspaceId] = [...previousLogs, ...uniqueLogsToAppend];
    } catch (fetchError) {
      error.value = fetchError instanceof Error ? fetchError.message : "Impossible de charger les logs";
    } finally {
      inFlightByWorkspaceId.value[workspaceId] = false;
      isLoading.value = false;
    }
  }

  return {
    byWorkspaceId,
    isLoading,
    error,
    appendDebugLog,
    clearWorkspaceLogs,
    fetchLogs,
  };
});
