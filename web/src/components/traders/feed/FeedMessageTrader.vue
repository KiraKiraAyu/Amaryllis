<script setup lang="ts">
import type { FeedMessage } from "@/composables/useTraderDetail"

const props = defineProps<{
  msg: FeedMessage
}>()

function decisionClass(decision: string): string {
  const d = decision.toUpperCase()
  if (d === "BUY" || d === "LONG")
    return "bg-emerald-100 dark:bg-emerald-900/30 text-emerald-600 dark:text-emerald-400"
  if (d === "SELL" || d === "SHORT")
    return "bg-rose-100 dark:bg-rose-900/30 text-rose-600 dark:text-rose-400"
  return "bg-surface-100 dark:bg-surface-800 text-surface-600 dark:text-surface-400"
}

</script>

<template>
  <div class="w-full flex flex-col gap-1.5 py-1">
    <!-- Header row -->
    <div class="flex items-center gap-2">
      <span class="pi text-xs pi-brain"></span>
      <span
        class="text-xs font-bold uppercase tracking-wider text-surface-500 dark:text-surface-400"
      >
        AI Trader
      </span>
    </div>

    <!-- Content (with typewriter cursor for streaming) -->
    <p
      v-if="props.msg.content"
      class="text-sm text-surface-700 dark:text-surface-200 whitespace-pre-wrap wrap-break-words leading-relaxed"
    >
      {{ props.msg.content
      }}<span
        v-if="props.msg.streaming"
        class="inline-block w-0.5 h-4 bg-primary-500 animate-pulse ml-0.5 align-middle"
      ></span>
    </p>

    <!-- Decision data -->
    <div
      v-if="props.msg.data?.decision"
      class="flex flex-wrap gap-2 mt-1"
    >
      <span
        class="text-xs px-2 py-0.5 rounded-lg font-semibold"
        :class="decisionClass(String(props.msg.data.decision))"
      >
        {{ props.msg.data.decision }}
      </span>
      <span
        v-if="props.msg.data.confidence"
        class="text-xs px-2 py-0.5 rounded-lg font-semibold bg-surface-100 dark:bg-surface-800 text-surface-600 dark:text-surface-400"
      >
        Conf: {{ props.msg.data.confidence }}%
      </span>
      <span
        v-if="props.msg.data.timeframe"
        class="text-xs px-2 py-0.5 rounded-lg font-semibold bg-surface-100 dark:bg-surface-800 text-surface-600 dark:text-surface-400"
      >
        {{ props.msg.data.timeframe }}
      </span>
    </div>
  </div>
</template>
