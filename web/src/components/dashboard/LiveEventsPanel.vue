<script setup lang="ts">
import type { DashboardLiveEvent } from "@/types/dashboard-ui"

defineProps<{
  events: DashboardLiveEvent[]
}>()

function eventIcon(type: string) {
  const icons: Record<string, string> = {
    equity_snapshot: "💹",
    position_update: "📌",
    trade_execution: "⚡",
    ai_decision: "🤖",
    engine_status: "⚙️",
    backtest_progress: "🔬",
    debate_message: "💬",
    debate_finished: "🏁",
    ping: "🫀",
  }
  return icons[type] ?? "📡"
}
</script>

<template>
  <div>
    <h2 class="font-bold text-sm">Live Events</h2>
    <div class="flex flex-col gap-2 max-h-55 overflow-auto">
      <div
        v-if="events.length === 0"
        class="text-center py-6 text-sm text-[--color-text-muted]"
      >
        Waiting for live events...
      </div>
      <div
        v-for="(event, index) in events"
        :key="index"
        class="flex items-start gap-3 py-2 px-3 rounded-lg text-xs bg-[--color-surface-elevated]"
      >
        <span class="shrink-0 mt-0.5">{{ eventIcon(event.type) }}</span>
        <div class="flex-1 min-w-0">
          <span class="font-semibold text-text-primary">
            {{ event.type }}
          </span>
          <span class="ml-2 font-mono truncate text-text-muted">
            {{ event.summary }}
          </span>
        </div>
        <span class="shrink-0 text-text-muted">{{ event.time }}</span>
      </div>
    </div>
  </div>
</template>
