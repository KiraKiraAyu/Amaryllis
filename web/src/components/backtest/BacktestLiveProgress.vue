<script setup lang="ts">
import Card from "primevue/card"
import type { BacktestLiveProgress } from "@/types/backtest-ui"

defineProps<{
  progress: BacktestLiveProgress
  progressPct: number
}>()
</script>

<template>
  <Card
    class="border border-primary bg-primary-50/5 dark:bg-primary-950/10 shadow-none! mb-4"
  >
    <template #content>
      <div class="flex items-center justify-between mb-4 flex-wrap gap-2">
        <h3 class="font-bold text-base text-surface-900 dark:text-white">
          Live Progress
          <span class="font-mono text-xs text-surface-400 dark:text-surface-500"
            >({{ progress.run_id?.toString().slice(0, 8) }})</span
          >
        </h3>
        <span
          class="text-xs font-semibold px-2 py-0.5 rounded bg-emerald-500/10 text-emerald-500 animate-pulse uppercase tracking-wider"
        >
          {{ progress.state }}
        </span>
      </div>

      <div class="grid grid-cols-2 sm:grid-cols-3 gap-3 text-xs mb-4">
        <div
          class="bg-surface-0 dark:bg-surface-900 p-2.5 rounded-xl border border-surface-200 dark:border-surface-800"
        >
          <span
            class="text-surface-400 dark:text-surface-500 block mb-0.5 font-semibold"
            >Processed Bars</span
          >
          <span
            class="font-mono text-base font-bold text-surface-800 dark:text-surface-200"
          >
            {{ progress.bar_index }} / {{ progress.total_bars }}
          </span>
        </div>
        <div
          class="bg-surface-0 dark:bg-surface-900 p-2.5 rounded-xl border border-surface-200 dark:border-surface-800"
        >
          <span
            class="text-surface-400 dark:text-surface-500 block mb-0.5 font-semibold"
            >Current Balance</span
          >
          <span
            class="font-mono text-base font-bold text-surface-800 dark:text-surface-200"
          >
            ${{
              (progress.equity ?? 0).toLocaleString(undefined, {
                minimumFractionDigits: 2,
                maximumFractionDigits: 2,
              })
            }}
          </span>
        </div>
        <div
          class="bg-surface-0 dark:bg-surface-900 p-2.5 rounded-xl border border-surface-200 dark:border-surface-800 col-span-2 sm:col-span-1"
        >
          <span
            class="text-surface-400 dark:text-surface-500 block mb-0.5 font-semibold"
            >Completion</span
          >
          <span class="font-mono text-base font-bold text-primary">
            {{ progressPct }}%
          </span>
        </div>
      </div>

      <!-- Custom styled progress bar -->
      <div
        class="h-2 rounded-full overflow-hidden bg-surface-100 dark:bg-surface-900 border border-surface-200/50 dark:border-surface-800/50"
      >
        <div
          class="h-full rounded-full transition-all duration-300 bg-linear-to-r from-reisa-pink-500 to-reisa-lilac-400"
          :style="{ width: progressPct + '%' }"
        ></div>
      </div>
    </template>
  </Card>
</template>
