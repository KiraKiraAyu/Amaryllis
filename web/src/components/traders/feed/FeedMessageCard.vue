<script setup lang="ts">
import type { FeedMessage } from "@/composables/useTraderDetail"
import type { PositionPayload } from "@/types/trading"

const props = defineProps<{
  msg: FeedMessage
}>()

function roleIcon(role: string): string {
  switch (role) {
    case "action":
      return "pi pi-bolt"
    case "position":
      return "pi pi-chart-line"
    default:
      return "pi pi-info-circle"
  }
}

function roleLabel(role: string): string {
  switch (role) {
    case "action":
      return "System Ops"
    case "position":
      return "Position"
    default:
      return "System Ops"
  }
}

function bubbleClass(role: string): string {
  switch (role) {
    case "action":
      return "bg-surface-100 dark:bg-surface-800/50 border border-surface-200 dark:border-surface-700"
    case "position":
      return "bg-blue-50/80 dark:bg-blue-500/5 border border-blue-200 dark:border-blue-800"
    default:
      return "bg-surface-100 dark:bg-surface-800/50 border border-surface-200 dark:border-surface-700"
  }
}

function pnlClass(val: number): string {
  return val >= 0
    ? "text-emerald-500 dark:text-emerald-400"
    : "text-rose-500 dark:text-rose-400"
}

function hasPayload(data: unknown): boolean {
  return (
    !!data &&
    typeof data === "object" &&
    Object.keys(data as object).length > 0
  )
}
</script>

<template>
  <div
    class="max-w-[85%] rounded-2xl px-4 py-3 flex flex-col gap-1.5"
    :class="bubbleClass(props.msg.role)"
  >
    <!-- Header row -->
    <div class="flex items-center gap-2">
      <span class="pi text-xs" :class="roleIcon(props.msg.role)"></span>
      <span
        class="text-xs font-bold uppercase tracking-wider text-surface-500 dark:text-surface-400"
      >
        {{ roleLabel(props.msg.role) }}
      </span>
    </div>

    <!-- Title -->
    <p class="text-sm font-bold text-surface-900 dark:text-white">
      {{ props.msg.title }}
    </p>

    <!-- Content -->
    <p
      v-if="props.msg.content"
      class="text-sm text-surface-600 dark:text-surface-300 whitespace-pre-wrap wrap-break-words"
    >
      {{ props.msg.content }}
    </p>

    <!-- Position data -->
    <div
      v-if="props.msg.data?.positions"
      class="flex flex-col gap-1 mt-1"
    >
      <div
        v-for="pos in props.msg.data.positions as PositionPayload[]"
        :key="pos.id"
        class="flex items-center gap-3 text-xs font-mono px-2 py-1 rounded-lg bg-surface-100/50 dark:bg-surface-800/50"
      >
        <span
          class="font-bold"
          :class="
            pos.side === 'LONG'
              ? 'text-emerald-500'
              : 'text-rose-500'
          "
          >{{ pos.side }}</span
        >
        <span class="font-bold text-surface-900 dark:text-white">{{
          pos.symbol
        }}</span>
        <span class="text-surface-500">qty: {{ pos.quantity }}</span>
        <span class="text-surface-500"
          >entry: ${{ pos.entry_price.toFixed(2) }}</span
        >
        <span class="text-surface-500"
          >mark: ${{ pos.mark_price.toFixed(2) }}</span
        >
        <span :class="pnlClass(pos.unrealized_pnl)">
          uPnL: {{ pos.unrealized_pnl >= 0 ? "+" : "" }}${{
            pos.unrealized_pnl.toFixed(2)
          }}
        </span>
      </div>
    </div>

    <!-- Event payload -->
    <details
      v-if="hasPayload(props.msg.data?.payload)"
      class="mt-1"
    >
      <summary
        class="text-xs text-surface-400 cursor-pointer hover:text-surface-600 dark:hover:text-surface-300 font-medium"
      >
        Show details
      </summary>
      <pre
        class="text-xs mt-1 p-2 rounded-lg overflow-auto max-h-32 font-mono bg-surface-100 dark:bg-surface-800 text-surface-600 dark:text-surface-400"
        >{{ JSON.stringify(props.msg.data?.payload, null, 2) }}</pre
      >
    </details>
  </div>
</template>
