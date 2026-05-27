import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { DocFile } from "../types";

export const useDocumentationStore = defineStore("documentation", () => {
  const docs = ref<DocFile[]>([]);
  const isLoading = ref(false);

  const hasDocs = computed(() => docs.value.length > 0);

  async function fetchDocs(workspaceId?: string) {
    isLoading.value = true;
    try {
      docs.value = await invoke<DocFile[]>("list_docs", { workspaceId });
    } catch (error) {
      console.error("Failed to fetch documentation files:", error);
    } finally {
      isLoading.value = false;
    }
  }

  async function readDocFile(path: string, workspaceId?: string): Promise<string> {
    try {
      return await invoke<string>("read_doc_file", { path, workspaceId });
    } catch (error) {
      console.error(`Failed to read doc file at ${path}:`, error);
      throw error;
    }
  }

  async function getDocImage(path: string, workspaceId?: string): Promise<Uint8Array> {
    try {
      const bytes = await invoke<number[]>("get_doc_image", { path, workspaceId });
      return new Uint8Array(bytes);
    } catch (error) {
      console.error(`Failed to get doc image at ${path}:`, error);
      throw error;
    }
  }

  return {
    docs,
    isLoading,
    hasDocs,
    fetchDocs,
    readDocFile,
    getDocImage,
  };
});
