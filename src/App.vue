<script setup lang="ts">
import { onMounted } from 'vue';
import { checkForAppUpdates } from './updater';

onMounted(() => {
  // Vérification automatique au démarrage (silencieuse si pas de mise à jour)
  checkForAppUpdates();
});
</script>

<template>
  <RouterView />
</template>

<style>
* {
  font-family: "Inter", "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
}
</style>