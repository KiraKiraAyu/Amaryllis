<script setup lang="ts">
import Button from "primevue/button"
import Select from "primevue/select"
import type { TraderPayload } from "@/types/trading"

defineProps<{
  traders: TraderPayload[]
}>()

const activeTrader = defineModel<string>({ required: true })

const emit = defineEmits<{
  refresh: []
}>()
</script>

<template>
  <div class="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
    <div>
      <h1 class="mt-1 text-3xl font-bold text-surface-900 dark:text-white">System Monitor</h1>
      <p class="mt-1 text-sm text-surface-500 dark:text-surface-400">
        Runtime metrics, alerts, and system events
      </p>
    </div>
    <div class="flex items-center gap-3 shrink-0">
      <Select
        v-model="activeTrader"
        :options="[{ id: '', name: '(All System)' }, ...traders]"
        placeholder="Select a Trader"
        optionLabel="name"
        optionValue="id"
        class="h-11 rounded-xl flex items-center w-48"
        @change="emit('refresh')"
      />
      <Button
        label="Refresh"
        icon="pi pi-refresh"
        severity="secondary"
        variant="outlined"
        class="rounded-xl h-11 px-4 cursor-pointer shrink-0"
        @click="emit('refresh')"
      />
    </div>
  </div>
</template>
