<script setup lang="ts">
import Button from "primevue/button"

const emit = defineEmits<{
  refresh: []
}>()
</script>

<template>
  <div class="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
    <div>
      <h1 class="mt-1 text-3xl font-bold text-surface-900 dark:text-white">Dashboard</h1>
      <p class="mt-1 text-sm text-surface-500 dark:text-surface-400">
        Real-time trading overview and performance metrics
      </p>
    </div>
    <div class="flex items-center gap-3">
      <Button
        icon="pi pi-refresh"
        label="Refresh"
        severity="secondary"
        variant="outlined"
        @click="emit('refresh')"
        class="rounded-xl h-11 px-4 cursor-pointer"
      />
    </div>
  </div>
</template>
