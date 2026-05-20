<script setup lang="ts">
defineProps<{
  title: string;
  value: string | number;
  subValue?: string;
  icon: string;
  color: string;
}>();
</script>

<template>
  <div class="stat-card" :style="{ borderColor: color }">
    <div class="icon-container" :style="{ color: color }">
      <span class="icon">{{ icon }}</span>
    </div>
    <div class="content">
      <span class="title">{{ title }}</span>
      <span class="value">{{ value }}</span>
      <span v-if="subValue" class="sub-value">{{ subValue }}</span>
    </div>
  </div>
</template>

<style scoped>
.stat-card {
  background-color: #161b22;
  border: 1px solid #30363d;
  border-radius: 12px;
  padding: 1.25rem;
  display: flex;
  align-items: center;
  gap: 1rem;
  flex: 1;
}

.icon-container {
  background-color: rgba(255, 255, 255, 0.05);
  border-radius: 10px;
  width: 48px;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 1.5rem;
  box-shadow: 0 0 15px rgba(0, 0, 0, 0.2);
}

.content {
  display: flex;
  flex-direction: column;
}

.title {
  color: #8b949e;
  font-size: 0.85rem;
}

.value {
  color: #f0f6fc;
  font-size: 1.5rem;
  font-weight: 700;
  line-height: 1.2;
}

.sub-value {
  color: #3fb950;
  font-size: 0.75rem;
  margin-top: 0.25rem;
}
</style>
