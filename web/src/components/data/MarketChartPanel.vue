<script setup lang="ts">
import CandlestickChart from "@/components/CandlestickChart.vue"
import type { CandlePoint } from "@/types/data-ui"

defineProps<{
  loading: boolean
  data: CandlePoint[]
  activeSymbol: string
}>()
</script>

<template>
  <div
    class="border border-surface-200 dark:border-surface-800 bg-surface-0 dark:bg-surface-900 rounded-xl shadow-none flex-1 flex flex-col min-h-0 relative overflow-hidden"
  >
    <div
      v-if="loading"
      class="flex-1 flex flex-col gap-2 items-center justify-center text-sm text-surface-400 dark:text-surface-500 w-full h-full"
    >
      <span class="pi pi-spin pi-spinner text-xl text-primary"></span>
      <span>Loading market data...</span>
    </div>
    <div v-else-if="data.length" class="flex-1 relative w-full h-full min-h-0">
      <CandlestickChart :data="data" class="absolute inset-0 w-full h-full" />
    </div>
    <div
      v-else
      class="flex-1 flex items-center justify-center text-sm text-surface-400 dark:text-surface-500 w-full h-full"
    >
      No market data available for {{ activeSymbol }}.
    </div>
  </div>
</template>
