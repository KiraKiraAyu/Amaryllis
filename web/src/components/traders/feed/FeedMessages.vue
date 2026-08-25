<script setup lang="ts">
import type { FeedMessage } from "@/composables/useTraderDetail"
import { formatTime } from "@/utils/format"

const props = defineProps<{
  msg: FeedMessage
  selected: boolean
}>()

const emit = defineEmits<{
  select: []
}>()

function roleLabel(role: string): string {
  switch (role) {
    case "trader":
      return "Trader"
    case "prompt":
      return "Prompt"
    case "warning":
      return "Warning"
    default:
      return "System"
  }
}

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
  <div
    class="p-3 h-14 flex flex-col rounded-lg cursor-pointer transition-colors duration-300 mb-0.5"
    :class="
      props.selected
        ? 'bg-surface-100 dark:bg-surface-950'
        : 'border-transparent hover:bg-surface-50 dark:hover:bg-surface-950'
    "
    @click="emit('select')"
  >
    <!-- Row 1: dot + time + label -->
    <div class="flex items-center gap-2">
      <span
        class="w-2 h-2 rounded-full shrink-0"
        :class="{
          'bg-primary-500':
            props.msg.role === 'trader' || props.msg.role === 'prompt',
          'bg-amber-500': props.msg.role === 'warning',
          'bg-surface-400': props.msg.role === 'system',
        }"
      ></span>
      <span class="text-xs font-mono text-surface-400">{{
        formatTime(props.msg.timestamp)
      }}</span>
      <span
        class="text-xs font-bold uppercase tracking-wider text-surface-500 dark:text-surface-400"
      >
        {{ roleLabel(props.msg.role) }}
      </span>
    </div>

    <!-- Row 2: brief summary -->
    <div
      v-if="props.msg.role === 'trader' && props.msg.data?.decision"
      class="flex items-center gap-1.5 pl-4 mt-0.5"
    >
      <span
        class="text-xs px-1.5 py-0.5 rounded font-semibold"
        :class="decisionClass(String(props.msg.data.decision))"
      >
        {{ props.msg.data.decision }}
      </span>
      <span
        v-if="props.msg.data?.symbol"
        class="text-xs font-mono text-surface-500 dark:text-surface-400 truncate"
      >
        {{ props.msg.data.symbol }}
      </span>
    </div>
    <div
      v-else-if="
        (props.msg.role === 'system' || props.msg.role === 'warning') &&
        props.msg.title
      "
      class="pl-4 mt-0.5"
    >
      <span
        class="text-xs text-surface-500 dark:text-surface-400 truncate block"
      >
        {{ props.msg.title }}
      </span>
    </div>
  </div>
</template>
