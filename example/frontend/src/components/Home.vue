<script setup>
import {onMounted, ref} from "vue";

const API_URL = 'http://localhost:4000'

const tasks = ref([])
const title = ref('')
const stats = ref(null)

async function loadTasks() {
  const res = await fetch(`${API_URL}/tasks`)
  tasks.value = await res.json()

  const statsRes = await fetch(`${API_URL}/stats`)
  stats.value = await statsRes.json()
}

async function addTask() {
  if (!title.value.trim()) return

  await fetch(`${API_URL}/tasks`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ title: title.value })
  })

  title.value = ''
  await loadTasks()
}

async function toggleTask(id) {
  await fetch(`${API_URL}/tasks/${id}/toggle`, {
    method: 'PATCH'
  })

  await loadTasks()
}

async function deleteTask(id) {
  await fetch(`${API_URL}/tasks/${id}`, {
    method: 'DELETE'
  })

  await loadTasks()
}

onMounted(loadTasks)
</script>

<template>
  <main style="font-family: sans-serif; max-width: 700px; margin: 40px auto;">
    <h1>Task Manager</h1>

    <form @submit.prevent="addTask" style="display: flex; gap: 8px;">
      <input
          v-model="title"
          placeholder="Nouvelle tâche"
          style="flex: 1; padding: 8px;"
      />

      <button type="submit">
        Ajouter
      </button>
    </form>

    <p v-if="stats">
      Total : {{ stats.tasksCount }} tâche(s)
    </p>

    <ul style="padding: 0; list-style: none;">
      <li
          v-for="task in tasks"
          :key="task.id"
          style="display: flex; gap: 8px; align-items: center; padding: 8px 0;"
      >
        <input
            type="checkbox"
            :checked="task.done"
            @change="toggleTask(task.id)"
        />

        <span :style="{ textDecoration: task.done ? 'line-through' : 'none', flex: 1 }">
            {{ task.title }}
          </span>

        <button @click="deleteTask(task.id)">
          Supprimer
        </button>
      </li>
    </ul>
  </main>
</template>