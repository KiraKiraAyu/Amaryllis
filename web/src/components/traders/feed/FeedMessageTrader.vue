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

function executionSummary(exec: Record<string, unknown>): string {
  const side = exec.side as string | undefined
  const symbol = exec.symbol as string | undefined
  const qty = exec.quantity as string | number | undefined
  const price = exec.price as number | undefined
  const parts: string[] = []
  if (side) parts.push(side)
  if (symbol) parts.push(symbol)
  if (qty != null) parts.push(`${qty} qty`)
  if (price != null) parts.push(`@ $${price}`)
  return parts.join(" · ")
}
</script>

<template>
  <div>
    <!-- Badges + execution summary -->
    <div class="flex flex-wrap items-center gap-2 mb-4">
      <span
        v-if="props.msg.data?.decision"
        class="text-xs px-2 py-0.5 rounded-lg font-semibold"
        :class="decisionClass(String(props.msg.data.decision))"
      >
        {{ props.msg.data.decision }}
      </span>
      <span
        v-if="props.msg.data?.confidence"
        class="text-xs px-2 py-0.5 rounded-lg font-semibold bg-surface-100 dark:bg-surface-800 text-surface-600 dark:text-surface-400"
      >
        Conf: {{ props.msg.data.confidence }}%
      </span>
      <span
        v-if="props.msg.data?.timeframe"
        class="text-xs px-2 py-0.5 rounded-lg font-semibold bg-surface-100 dark:bg-surface-800 text-surface-600 dark:text-surface-400"
      >
        {{ props.msg.data.timeframe }}
      </span>
      <span
        v-if="props.msg.data?.execution"
        class="text-xs font-mono text-surface-600 dark:text-surface-300"
      >
        {{ executionSummary(props.msg.data.execution as Record<string, unknown>) }}
      </span>
    </div>

    <!-- Reasoning -->
    <div v-if="props.msg.content" class="mb-4">
      <div
        class="text-xs font-bold uppercase tracking-wider text-surface-400 mb-1"
      >
        Reasoning
      </div>
      <p
        class="text-sm text-surface-700 dark:text-surface-200 whitespace-pre-wrap wrap-break-words leading-relaxed"
      >
        {{ props.msg.content
        }}<span
          v-if="props.msg.streaming"
          class="inline-block w-0.5 h-4 bg-primary-500 animate-pulse ml-0.5 align-middle"
        ></span>
      </p>
    </div>

    <!-- Execution -->
    <div v-if="props.msg.data?.execution">
      <div
        class="text-xs font-bold uppercase tracking-wider text-surface-400 mb-1"
      >
        Execution
      </div>
      <div
        class="text-xs font-mono p-3 rounded-lg bg-surface-100 dark:bg-surface-800 text-surface-700 dark:text-surface-200 overflow-x-auto"
      >
        {{ JSON.stringify(props.msg.data.execution, null, 2) }}
      </div>
    </div>
  </div>
</template>
