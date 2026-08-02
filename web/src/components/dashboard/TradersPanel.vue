<script setup lang="ts">
import Card from "primevue/card"
import TraderRow from "@/components/TraderRow.vue"
import type { DashboardTrader } from "@/types/dashboard-ui"

defineProps<{
  traders: DashboardTrader[]
  loading: boolean
  initialLoadDone: boolean
}>()

const emit = defineEmits<{
  start: [id: string]
  stop: [id: string]
  sync: [id: string]
}>()
</script>

<template>
  <Card class="border border-surface-200 dark:border-surface-800 bg-surface-0 dark:bg-surface-900 shadow-none!">
    <template #content>
      <div class="flex items-center justify-between mb-4">
        <h2 class="font-bold text-lg text-surface-900 dark:text-white">Active Traders</h2>
      </div>

      <div
        v-if="!initialLoadDone && loading"
        class="text-center text-sm py-12 text-surface-400 dark:text-surface-500"
      >
        <span class="pi pi-spin pi-spinner mr-2"></span>
        Loading traders...
      </div>
      <div v-else-if="traders.length === 0" class="text-center py-12 border border-dashed border-surface-200 dark:border-surface-800 rounded-2xl bg-surface-50/50 dark:bg-surface-950/20">
        <p class="text-sm text-surface-400 dark:text-surface-500">
          No traders active yet. Create one to start trading.
        </p>
      </div>
      <div v-else class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <TraderRow
          v-for="trader in traders"
          :key="trader.id"
          :trader="trader"
          @start="emit('start', trader.id)"
          @stop="emit('stop', trader.id)"
          @sync="emit('sync', trader.id)"
        />
      </div>
    </template>
  </Card>
</template>
