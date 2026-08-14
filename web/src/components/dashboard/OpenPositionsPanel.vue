<script setup lang="ts">
import Card from "primevue/card"
import PositionsTable from "@/components/common/PositionsTable.vue"
import type { DashboardPosition } from "@/types/dashboard-ui"

defineProps<{
  positions: DashboardPosition[]
  traderName: (id: string) => string
}>()

const emit = defineEmits<{
  close: [traderId: string, symbol: string, side: string]
}>()
</script>

<template>
  <Card
    class="border border-surface-200 dark:border-surface-800 bg-surface-0 dark:bg-surface-900 shadow-none!"
  >
    <template #content>
      <div class="flex items-center justify-between mb-4">
        <h2 class="font-bold text-lg text-surface-900 dark:text-white">
          Open Positions
        </h2>
        <span
          class="text-xs font-semibold px-2.5 py-1 bg-surface-100 dark:bg-surface-800 rounded-lg text-surface-600 dark:text-surface-400"
        >
          {{ positions.length }} active
        </span>
      </div>

      <PositionsTable
        :positions="positions"
        :show-trader="true"
        :trader-name="traderName"
        :closable="true"
        scroll-height="440px"
        @close="(traderId, symbol, side) => emit('close', traderId, symbol, side)"
      />
    </template>
  </Card>
</template>
