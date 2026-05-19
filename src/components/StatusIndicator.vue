<script setup lang="ts">
import type { ServiceRuntimeStatus } from "../types";

defineProps<{
  status: ServiceRuntimeStatus;
  showText?: boolean;
}>();

const getStatusColor = (status: ServiceRuntimeStatus) => {
  switch (status) {
    case "running":
      return "#22c55e"; // Vert
    case "starting":
    case "stopping":
      return "#f59e0b"; // Orange
    case "failed":
    case "blocked":
      return "#ef4444"; // Rouge
    case "idle":
    case "stopped":
    default:
      return "#94a3b8"; // Gris
  }
};

const getStatusLabel = (status: ServiceRuntimeStatus) => {
  switch (status) {
    case "starting":
      return "chargement";
    case "stopping":
      return "arrêt en cours";
    case "running":
      return "en cours";
    case "failed":
      return "échec";
    case "blocked":
      return "bloqué";
    case "stopped":
      return "arrêté";
    case "idle":
      return "inactif";
    default:
      return status;
  }
};
</script>

<template>
  <div class="status-indicator" :title="getStatusLabel(status)">
    <span
      class="dot"
      :style="{ backgroundColor: getStatusColor(status) }"
      :class="{ 'pulse': status === 'starting' || status === 'stopping' }"
    ></span>
    <span v-if="showText" class="label">{{ getStatusLabel(status) }}</span>
  </div>
</template>

<style scoped>
.status-indicator {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
}

.dot {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  display: inline-block;
  flex-shrink: 0;
}

.label {
  font-size: 0.9rem;
  font-weight: 500;
  color: #4f5d75;
}

.pulse {
  animation: pulse-animation 1.5s infinite ease-in-out;
}

@keyframes pulse-animation {
  0% {
    opacity: 1;
    transform: scale(1);
  }
  50% {
    opacity: 0.5;
    transform: scale(1.1);
  }
  100% {
    opacity: 1;
    transform: scale(1);
  }
}
</style>
