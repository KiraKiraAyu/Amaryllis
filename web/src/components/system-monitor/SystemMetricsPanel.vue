<script setup lang="ts">
import StatCard from "@/components/StatCard.vue"
import type { RuntimeMetricsPayload } from "@/types/trading"

defineProps<{
  metrics: RuntimeMetricsPayload | null
  loading: boolean
  fmt: (value: number) => string
}>()
</script>

<template>
  <div class="flex flex-col gap-6 lg:col-span-2">
    <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
      <StatCard
        title="Total Events (24h)"
        :value="metrics?.totals?.replace_succeeded ?? 0"
        :loading="loading"
      />
      <StatCard
        title="Open Submissions"
        :value="metrics?.totals?.open_submitted ?? 0"
        :loading="loading"
      />
      <StatCard
        title="Stale Reconcile"
        :value="metrics?.totals?.stale_reconcile_terminal ?? 0"
        :loading="loading"
      />
      <StatCard
        title="Rate Limits Hits"
        :value="metrics?.totals?.replace_throttled ?? 0"
        :loading="loading"
      />
    </div>

    <div class="flex flex-col gap-4">
      <h2 class="font-bold text-sm text-surface-900 dark:text-white">
        Health Metrics
      </h2>
      <div v-if="loading" class="text-xs text-center text-surface-500 py-4">
        Loading metrics...
      </div>
      <div
        v-else-if="metrics?.rates_pct"
        class="grid grid-cols-1 md:grid-cols-3 gap-4"
      >
        <div
          class="p-4 rounded-xl border border-surface-200 dark:border-surface-800 bg-surface-0 dark:bg-surface-900"
        >
          <div class="text-xs mb-1 text-surface-500 font-medium">
            Open Market Fallback
          </div>
          <div
            class="text-xl font-mono font-bold text-surface-900 dark:text-white"
          >
            {{ fmt(metrics.rates_pct.open_market_fallback_rate) }}%
          </div>
        </div>
        <div
          class="p-4 rounded-xl border border-surface-200 dark:border-surface-800 bg-surface-0 dark:bg-surface-900"
        >
          <div class="text-xs mb-1 text-surface-500 font-medium">
            Replace Throttle Rate
          </div>
          <div
            class="text-xl font-mono font-bold text-surface-900 dark:text-white"
          >
            {{ fmt(metrics.rates_pct.replace_throttle_rate) }}%
          </div>
        </div>
        <div
          class="p-4 rounded-xl border border-surface-200 dark:border-surface-800 bg-surface-0 dark:bg-surface-900"
        >
          <div class="text-xs mb-1 text-surface-500 font-medium">
            Terminal Reconcile Rate
          </div>
          <div
            class="text-xl font-mono font-bold text-surface-900 dark:text-white"
          >
            {{ fmt(metrics.rates_pct.stale_reconcile_terminal_rate) }}%
          </div>
        </div>
      </div>
      <div v-else class="text-xs text-center text-surface-500 py-4">
        No metrics available
      </div>
    </div>

    <slot />
  </div>
</template>
