<script setup lang="ts">
import Card from "primevue/card"
import ProgressSpinner from "primevue/progressspinner"
import PositionsTable from "@/components/common/PositionsTable.vue"
import type { PositionPayload } from "@/types/trading"

defineProps<{
  positions: PositionPayload[]
  loading?: boolean
}>()
</script>

<template>
  <Card
    class="border border-surface-200 dark:border-surface-800 bg-surface-0 dark:bg-surface-900 shadow-none!"
  >
    <template #content>
      <div class="flex items-center justify-between mb-4">
        <div class="flex items-center gap-2">
          <h2 class="font-bold text-sm text-surface-900 dark:text-white">
            Open Positions
          </h2>
          <span
            class="text-xs font-semibold px-2 py-0.5 bg-primary-500/10 text-primary rounded-lg"
          >
            {{ positions.length }} active
          </span>
        </div>
        <ProgressSpinner
          v-if="loading"
          style="width: 1.25rem; height: 1.25rem"
          strokeWidth="4"
          class="text-surface-400"
        />
      </div>

      <PositionsTable
        :positions="positions"
        :show-trader="true"
        scroll-height="360px"
      />
    </template>
  </Card>
</template>
