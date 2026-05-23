<script setup lang="ts">
import { computed, ref } from "vue";
import { useRoute } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { Terminal } from "@wterm/vue";
import "@wterm/vue/css";

const route = useRoute();
const terminalRef = ref<InstanceType<typeof Terminal> | null>(null);
const commandBuffer = ref("");
const isRunning = ref(false);

const workspaceId = computed(() => String(route.query.workspaceId || ""));
const serviceName = computed(() => String(route.query.serviceName || ""));

function writeLine(text: string) {
  terminalRef.value?.write(`${text}\r\n`);
}

function writePrompt() {
  terminalRef.value?.write("$ ");
}

function onReady() {
  writeLine(`Terminal Docker: ${workspaceId.value}/${serviceName.value}`);
  writeLine("Tapez une commande et validez avec Entrée.");
  writePrompt();
}

async function runCommand(command: string) {
  if (!workspaceId.value || !serviceName.value) {
    writeLine("Paramètres de terminal invalides.");
    return;
  }

  isRunning.value = true;
  try {
    const output = await invoke<string>("run_container_command", {
      workspaceId: workspaceId.value,
      serviceName: serviceName.value,
      command,
    });
    if (output.trim().length > 0) {
      writeLine(output.replace(/\n/g, "\r\n"));
    }
  } catch (error) {
    writeLine(`Erreur: ${String(error)}`);
  } finally {
    isRunning.value = false;
  }
}

async function onData(chunk: string) {
  if (isRunning.value) {
    return;
  }

  if (chunk === "\r") {
    const cmd = commandBuffer.value.trim();
    terminalRef.value?.write("\r\n");
    commandBuffer.value = "";
    if (cmd.length === 0) {
      writePrompt();
      return;
    }

    await runCommand(cmd);
    writePrompt();
    return;
  }

  if (chunk === "\u007f") {
    if (commandBuffer.value.length > 0) {
      commandBuffer.value = commandBuffer.value.slice(0, -1);
      terminalRef.value?.write("\b \b");
    }
    return;
  }

  commandBuffer.value += chunk;
  terminalRef.value?.write(chunk);
}
</script>

<template>
  <div class="terminal-view">
    <Terminal
      ref="terminalRef"
      class="terminal"
      auto-resize
      :cursor-blink="true"
      @ready="onReady"
      @data="onData"
    />
  </div>
</template>

<style scoped>
.terminal-view {
  width: 100%;
  height: 100vh;
  background: #0d1117;
  padding: 0;
  box-sizing: border-box;
}

.terminal {
  width: 100%;
  height: 100%;
}
</style>
