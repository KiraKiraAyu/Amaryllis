<script setup lang="ts">
import Select from "primevue/select"
import Button from "primevue/button"

defineProps<{
  symbols: string[]
}>()

const activeSymbol = defineModel<string>("symbol", { required: true })
const activeInterval = defineModel<string>("interval", { required: true })

const emit = defineEmits<{
  refresh: []
}>()

const intervals = ["1m", "5m", "15m", "1h", "4h", "1d"]
</script>

<template>
  <div class="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
    <div>
      <h1 class="mt-1 text-3xl font-bold text-surface-900 dark:text-white">Market Data</h1>
      <p class="mt-1 text-sm text-surface-500 dark:text-surface-400">
        Live candlestick charts and exchange symbol histories
      </p>
    </div>
    <div class="flex items-center gap-2 sm:gap-3 shrink-0">
      <Select
        v-model="activeSymbol"
        :options="symbols"
        placeholder="Select symbol..."
        @change="emit('refresh')"
        class="h-11 rounded-xl flex items-center w-36 text-xs font-mono font-bold"
      />
      <Select
        v-model="activeInterval"
        :options="intervals"
        placeholder="Interval"
        @change="emit('refresh')"
        class="h-11 rounded-xl flex items-center w-24 text-xs font-semibold"
      />
      <Button
        icon="pi pi-refresh"
        label="Refresh"
        severity="secondary"
        variant="outlined"
        @click="emit('refresh')"
        class="rounded-xl h-11 px-4 cursor-pointer shrink-0"
      />
    </div>
  </div>
</template>
