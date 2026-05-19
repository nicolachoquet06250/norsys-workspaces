import {defineStore} from "pinia";
import {invoke} from "@tauri-apps/api/core";
import {ref} from "vue";

export const useLogsStore = defineStore("logs", () => {
  const byWorkspaceId = ref<Record<string, string[]>>({});
  const isLoading = ref(false);
  const error = ref<string | null>(null);
  const inFlightByWorkspaceId = ref<Record<string, boolean>>({});
  const seenLogsByWorkspaceId = ref<Record<string, Set<string>>>( {});

  function extractTimestamp(logLine: string): string | null {
    // 1. Format ISO bracketé: [2026-05-19T16:29:00Z]
    const bracketedIsoMatch = logLine.match(/\[(\d{4}-\d{2}-\d{2}T[^\]]+)]/);
    if (bracketedIsoMatch?.[1]) {
      return bracketedIsoMatch[1];
    }

    // 2. Format Apache/PHP: [Tue May 19 14:26:08 2026]
    // Gère les jours à un chiffre avec espace: [Tue May  9 14:26:08 2026]
    const apacheLogMatch = logLine.match(
      /\[([A-Z][a-z]{2} [A-Z][a-z]{2} +?\d{1,2} \d{2}:\d{2}:\d{2} \d{4})]/,
    );
    if (apacheLogMatch?.[1]) {
      return apacheLogMatch[1];
    }

    // 3. Common Log Format (Nginx/Apache access): [19/May/2026:14:26:08 +0200]
    const commonLogFormatMatch = logLine.match(
      /\[(\d{1,2}\/[A-Z][a-z]{2}\/\d{4}:\d{2}:\d{2}:\d{2} [+-]\d{4})]/,
    );
    if (commonLogFormatMatch?.[1]) {
      return commonLogFormatMatch[1];
    }

    // 4. Format date/heure simple (ISO ou proche) sans crochets
    const plainDateTimeMatch = logLine.match(
      /\b(\d{4}-\d{2}-\d{2}[ T]\d{2}:\d{2}:\d{2}(?:[.,]\d+)?(?:Z|[+-]\d{2}(?::?\d{2})?)?)\b/,
    );
    if (plainDateTimeMatch?.[1]) {
      return plainDateTimeMatch[1];
    }

    // fallback pour les logs de type "HEAD /" qui pourraient avoir un timestamp différent ou pas de timestamp
    // mais ici on extrait le timestamp pour l'affichage, pas pour le dédoublage
    return null;
  }

  function appendDebugLog(workspaceId: string, message: string) {
    const timestamp = new Date().toISOString();
    const previousLogs = byWorkspaceId.value[workspaceId] ?? [];
    const logLine = `[DEBUG][${timestamp}] ${message}`;
    byWorkspaceId.value[workspaceId] = [...previousLogs, logLine];
    
    if (!seenLogsByWorkspaceId.value[workspaceId]) {
      seenLogsByWorkspaceId.value[workspaceId] = new Set();
    }
    seenLogsByWorkspaceId.value[workspaceId].add(logLine);
  }

  function clearWorkspaceLogs(workspaceId: string) {
    delete byWorkspaceId.value[workspaceId];
    delete seenLogsByWorkspaceId.value[workspaceId];
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

      if (!seenLogsByWorkspaceId.value[workspaceId]) {
        seenLogsByWorkspaceId.value[workspaceId] = new Set();
      }
      const seenLogs = seenLogsByWorkspaceId.value[workspaceId];

      for (const previousLog of previousLogs) {
        seenLogs.add(previousLog);
      }

      const uniqueLogsToAppend: string[] = [];
      for (const fetchedLog of fetchedLogs) {
        if (seenLogs.has(fetchedLog)) {
          continue;
        }

        // Filtrer les requêtes de healthcheck (HEAD /) demandées par l'utilisateur
        if (fetchedLog.includes("HEAD /") || fetchedLog.includes("Accepted") || fetchedLog.includes("Closing")) {
          continue;
        }

        seenLogs.add(fetchedLog);
        uniqueLogsToAppend.push(fetchedLog);
      }

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
    extractTimestamp,
    appendDebugLog,
    clearWorkspaceLogs,
    fetchLogs,
  };
});
