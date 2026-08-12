<script setup lang="ts">
import Card from "primevue/card"
import type { StrategyTestResult } from "@/types/strategy-ui"

defineProps<{
  result: StrategyTestResult
}>()
</script>

<template>
  <Card
    class="border border-surface-200 dark:border-surface-800 bg-surface-0 dark:bg-surface-900 shadow-none! mt-4"
  >
    <template #content>
      <h3 class="font-bold text-base mb-4 text-surface-900 dark:text-white">
        AI Test Run Results
      </h3>
      <div class="flex flex-col gap-2">
        <div
          v-for="(decision, index) in result.decisions"
          :key="index"
          class="flex items-center gap-3 py-2.5 px-4 rounded-xl text-xs bg-surface-50 dark:bg-surface-950 border border-surface-200 dark:border-surface-800"
        >
          <span
            class="font-black uppercase text-xs px-2 py-0.5 rounded-md"
            :class="
              decision.action.toLowerCase() === 'buy' ||
              decision.action.toLowerCase() === 'long'
                ? 'bg-emerald-500/15 text-emerald-600 dark:text-emerald-400'
                : decision.action.toLowerCase() === 'sell' ||
                    decision.action.toLowerCase() === 'short'
                  ? 'bg-rose-500/15 text-rose-600 dark:text-rose-400'
                  : 'bg-surface-100 text-surface-600 dark:bg-surface-800 dark:text-surface-400'
            "
          >
            {{ decision.action }}
          </span>
          <span class="font-mono font-bold text-surface-900 dark:text-white">{{
            decision.symbol
          }}</span>
          <span class="text-surface-400 dark:text-surface-500 font-medium">
            Conf: {{ decision.confidence }}%
          </span>
          <span
            class="flex-1 text-surface-600 dark:text-surface-300 truncate font-medium"
          >
            {{ decision.reasoning }}
          </span>
          <span
            class="text-surface-400 dark:text-surface-500 font-mono text-xs"
          >
            {{ result.duration_ms }}ms
          </span>
        </div>
      </div>
      <div v-if="result.ai_response" class="mt-4">
        <label class="text-xs font-bold text-surface-500 block mb-1.5"
          >Raw AI Response</label
        >
        <pre
          class="text-xs p-3 rounded-xl overflow-auto max-h-40 font-mono bg-surface-50 dark:bg-surface-950 text-surface-600 dark:text-surface-400 border border-surface-200 dark:border-surface-800"
          >{{ result.ai_response }}</pre
        >
      </div>
    </template>
  </Card>
</template>
