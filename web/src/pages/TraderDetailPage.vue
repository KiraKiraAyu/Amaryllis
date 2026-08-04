<script setup lang="ts">
import { computed, onMounted, ref, watch, nextTick } from "vue"
import { useRoute, useRouter } from "vue-router"
import Button from "primevue/button"
import Card from "primevue/card"
import ProgressSpinner from "primevue/progressspinner"
import PageHeader from "@/components/layout/PageHeader.vue"
import { useTraderDetail } from "@/composables/useTraderDetail"
import { fmtUsd } from "@/utils/format"

const route = useRoute()
const router = useRouter()

const traderId = computed(() => route.params.id as string)
const {
  trader,
  account,
  positions,
  loading,
  error,
  feed,
  typing,
  loadAll,
  startTrader,
  stopTrader,
} = useTraderDetail(traderId)

const feedContainer = ref<HTMLElement | null>(null)

// Auto-scroll to bottom on new messages
watch(
  () => feed.value.length,
  async () => {
    await nextTick()
    if (feedContainer.value) {
      feedContainer.value.scrollTop = feedContainer.value.scrollHeight
    }
  },
)

// Reload when trader ID changes
watch(traderId, () => loadAll(), { immediate: false })

onMounted(loadAll)

function backToList() {
  router.push({ name: "traders" })
}

function roleColor(role: string): string {
  switch (role) {
    case "trader":
      return "border-l-primary-500 bg-primary-50/50 dark:bg-primary-500/5"
    case "action":
      return "border-l-amber-500 bg-amber-50/50 dark:bg-amber-500/5"
    case "position":
      return "border-l-blue-500 bg-blue-50/50 dark:bg-blue-500/5"
    default:
      return "border-l-surface-400 bg-surface-50/50 dark:bg-surface-900/30"
  }
}

function roleIcon(role: string): string {
  switch (role) {
    case "trader":
      return "pi pi-brain"
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
    case "trader":
      return "AI Trader"
    case "action":
      return "Action"
    case "position":
      return "Position"
    default:
      return "System"
  }
}

function formatTime(ts: number): string {
  return new Date(ts).toLocaleTimeString("en-US", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  })
}

function pnlClass(val: number): string {
  return val >= 0
    ? "text-emerald-500 dark:text-emerald-400"
    : "text-rose-500 dark:text-rose-400"
}
</script>

<template>
  <div class="flex flex-col gap-6 h-full">
    <!-- Header -->
    <PageHeader
      :title="trader?.name || 'Trader Detail'"
      :description="trader ? `Model: ${trader.ai_model_id} · Exchange: ${trader.exchange_id}` : 'Loading...'"
    >
      <template #actions>
        <div class="flex items-center gap-3">
          <Button
            icon="pi pi-arrow-left"
            label="Back"
            severity="secondary"
            variant="outlined"
            class="rounded-xl h-11 px-4 cursor-pointer"
            @click="backToList"
          />
          <Button
            v-if="trader && !trader.is_running"
            icon="pi pi-play"
            label="Start"
            severity="success"
            class="rounded-xl h-11 px-4 cursor-pointer bg-emerald-500! border-emerald-500! hover:bg-emerald-600! hover:border-emerald-600! text-white!"
            @click="startTrader"
          />
          <Button
            v-else-if="trader && trader.is_running"
            icon="pi pi-stop"
            label="Stop"
            severity="danger"
            class="rounded-xl h-11 px-4 cursor-pointer bg-rose-500! border-rose-500! hover:bg-rose-600! hover:border-rose-600! text-white!"
            @click="stopTrader"
          />
        </div>
      </template>
    </PageHeader>

    <!-- Loading -->
    <div v-if="loading" class="flex items-center justify-center py-20">
      <ProgressSpinner />
    </div>

    <!-- Error -->
    <div v-else-if="error" class="text-center py-20">
      <p class="text-rose-500">{{ error }}</p>
    </div>

    <!-- Main content: info bar + chat feed -->
    <div v-else-if="trader" class="flex flex-col gap-4 flex-1 min-h-0">
      <!-- Stats bar -->
      <div class="flex items-center gap-6 p-4 rounded-2xl bg-surface-50 dark:bg-surface-950 border border-surface-200 dark:border-surface-800 flex-wrap">
        <!-- Status -->
        <div class="flex items-center gap-2">
          <span class="relative flex h-2.5 w-2.5">
            <span
              v-if="trader.is_running"
              class="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"
            ></span>
            <span
              class="relative inline-flex rounded-full h-2.5 w-2.5"
              :class="trader.is_running ? 'bg-emerald-500' : 'bg-surface-400'"
            ></span>
          </span>
          <span class="text-sm font-bold" :class="trader.is_running ? 'text-emerald-500' : 'text-surface-500'">
            {{ trader.is_running ? "Running" : "Stopped" }}
          </span>
        </div>

        <div class="w-px h-6 bg-surface-200 dark:bg-surface-800"></div>

        <!-- Balance -->
        <div class="flex flex-col">
          <span class="text-xs text-surface-400 font-bold uppercase tracking-wider">Balance</span>
          <span class="text-sm font-bold font-mono text-surface-900 dark:text-white">
            ${{ fmtUsd(account?.total_balance ?? trader.initial_balance) }}
          </span>
        </div>

        <!-- Unrealized PnL -->
        <div class="flex flex-col" v-if="account">
          <span class="text-xs text-surface-400 font-bold uppercase tracking-wider">Unrealized PnL</span>
          <span class="text-sm font-bold font-mono" :class="pnlClass(account.unrealized_pnl)">
            {{ account.unrealized_pnl >= 0 ? "+" : "" }}${{ fmtUsd(account.unrealized_pnl) }}
          </span>
        </div>

        <!-- Realized PnL -->
        <div class="flex flex-col" v-if="account">
          <span class="text-xs text-surface-400 font-bold uppercase tracking-wider">Realized PnL</span>
          <span class="text-sm font-bold font-mono" :class="pnlClass(account.realized_pnl)">
            {{ account.realized_pnl >= 0 ? "+" : "" }}${{ fmtUsd(account.realized_pnl) }}
          </span>
        </div>

        <!-- Open Positions -->
        <div class="flex flex-col">
          <span class="text-xs text-surface-400 font-bold uppercase tracking-wider">Positions</span>
          <span class="text-sm font-bold font-mono text-surface-900 dark:text-white">{{ positions.length }}</span>
        </div>

        <!-- Budget -->
        <div class="flex flex-col">
          <span class="text-xs text-surface-400 font-bold uppercase tracking-wider">Budget</span>
          <span class="text-sm font-bold font-mono text-surface-900 dark:text-white">${{ fmtUsd(trader.initial_balance) }}</span>
        </div>

        <!-- Scan Interval -->
        <div class="flex flex-col">
          <span class="text-xs text-surface-400 font-bold uppercase tracking-wider">Scan Interval</span>
          <span class="text-sm font-bold font-mono text-surface-900 dark:text-white">{{ trader.scan_interval_minutes }}m</span>
        </div>
      </div>

      <!-- Open positions summary -->
      <div v-if="positions.length > 0" class="flex flex-wrap gap-2">
        <div
          v-for="pos in positions"
          :key="pos.id"
          class="flex items-center gap-2 px-3 py-1.5 rounded-xl text-xs font-mono border"
          :class="pos.side === 'LONG'
            ? 'border-emerald-200 dark:border-emerald-800 bg-emerald-50 dark:bg-emerald-950/20'
            : 'border-rose-200 dark:border-rose-800 bg-rose-50 dark:bg-rose-950/20'"
        >
          <span class="font-bold" :class="pos.side === 'LONG' ? 'text-emerald-600 dark:text-emerald-400' : 'text-rose-600 dark:text-rose-400'">
            {{ pos.side }}
          </span>
          <span class="font-bold text-surface-900 dark:text-white">{{ pos.symbol }}</span>
          <span class="text-surface-500">{{ pos.quantity }}</span>
          <span class="text-surface-400">@ ${{ pos.entry_price.toFixed(2) }}</span>
          <span :class="pnlClass(pos.unrealized_pnl)">
            {{ pos.unrealized_pnl >= 0 ? "+" : "" }}${{ pos.unrealized_pnl.toFixed(2) }}
          </span>
        </div>
      </div>

      <!-- Chat feed -->
      <Card class="flex-1 min-h-0 border border-surface-200 dark:border-surface-800 bg-surface-0 dark:bg-surface-900 shadow-none!">
        <template #content>
          <div ref="feedContainer" class="flex flex-col gap-4 overflow-y-auto h-full p-1 max-h-[calc(100vh-380px)]">
            <!-- Empty state -->
            <div v-if="feed.length === 0" class="flex flex-col items-center justify-center h-full gap-3 text-surface-400">
              <span class="pi pi-comments text-4xl"></span>
              <p class="text-sm">No activity yet. Start the trader to see live AI decisions and actions.</p>
            </div>

            <!-- Messages -->
            <div
              v-for="msg in feed"
              :key="msg.id"
              class="flex flex-col gap-1.5 border-l-3 pl-4 pr-3 py-3 rounded-r-xl rounded-l-sm"
              :class="roleColor(msg.role)"
            >
              <!-- Message header -->
              <div class="flex items-center gap-2">
                <span class="pi text-sm" :class="roleIcon(msg.role)"></span>
                <span class="text-xs font-bold uppercase tracking-wider text-surface-600 dark:text-surface-400">
                  {{ roleLabel(msg.role) }}
                </span>
                <span class="text-xs text-surface-400 font-mono ml-auto">
                  {{ formatTime(msg.timestamp) }}
                </span>
              </div>

              <!-- Title -->
              <p class="text-sm font-bold text-surface-900 dark:text-white">
                {{ msg.title }}
              </p>

              <!-- Content (with typewriter cursor for streaming) -->
              <p v-if="msg.content" class="text-sm text-surface-600 dark:text-surface-300 whitespace-pre-wrap break-words">
                {{ msg.content
                }}<span
                  v-if="msg.streaming"
                  class="inline-block w-0.5 h-4 bg-primary-500 animate-pulse ml-0.5 align-middle"
                ></span>
              </p>

              <!-- Decision data -->
              <div v-if="msg.data?.decision" class="flex flex-wrap gap-2 mt-1">
                <span class="text-xs px-2 py-0.5 rounded-lg font-semibold"
                  :class="String(msg.data.decision).toUpperCase() === 'BUY' || String(msg.data.decision).toUpperCase() === 'LONG'
                    ? 'bg-emerald-100 dark:bg-emerald-900/30 text-emerald-600 dark:text-emerald-400'
                    : String(msg.data.decision).toUpperCase() === 'SELL' || String(msg.data.decision).toUpperCase() === 'SHORT'
                    ? 'bg-rose-100 dark:bg-rose-900/30 text-rose-600 dark:text-rose-400'
                    : 'bg-surface-100 dark:bg-surface-800 text-surface-600 dark:text-surface-400'"
                >
                  {{ msg.data.decision }}
                </span>
                <span v-if="msg.data.confidence" class="text-xs px-2 py-0.5 rounded-lg font-semibold bg-surface-100 dark:bg-surface-800 text-surface-600 dark:text-surface-400">
                  Conf: {{ msg.data.confidence }}%
                </span>
                <span v-if="msg.data.timeframe" class="text-xs px-2 py-0.5 rounded-lg font-semibold bg-surface-100 dark:bg-surface-800 text-surface-600 dark:text-surface-400">
                  {{ msg.data.timeframe }}
                </span>
              </div>

              <!-- Position data -->
              <div v-if="msg.data?.positions" class="flex flex-col gap-1 mt-1">
                <div
                  v-for="pos in (msg.data.positions as any[])"
                  :key="pos.id"
                  class="flex items-center gap-3 text-xs font-mono px-2 py-1 rounded-lg bg-surface-100/50 dark:bg-surface-800/50"
                >
                  <span class="font-bold" :class="pos.side === 'LONG' ? 'text-emerald-500' : 'text-rose-500'">{{ pos.side }}</span>
                  <span class="font-bold text-surface-900 dark:text-white">{{ pos.symbol }}</span>
                  <span class="text-surface-500">qty: {{ pos.quantity }}</span>
                  <span class="text-surface-500">entry: ${{ pos.entry_price.toFixed(2) }}</span>
                  <span class="text-surface-500">mark: ${{ pos.mark_price.toFixed(2) }}</span>
                  <span :class="pnlClass(pos.unrealized_pnl)">
                    uPnL: {{ pos.unrealized_pnl >= 0 ? "+" : "" }}${{ pos.unrealized_pnl.toFixed(2) }}
                  </span>
                </div>
              </div>

              <!-- Event payload (for system/action messages) -->
              <details v-if="msg.data?.payload && Object.keys(msg.data.payload as object).length > 0" class="mt-1">
                <summary class="text-xs text-surface-400 cursor-pointer hover:text-surface-600 dark:hover:text-surface-300 font-medium">
                  Show details
                </summary>
                <pre class="text-xs mt-1 p-2 rounded-lg overflow-auto max-h-32 font-mono bg-surface-100 dark:bg-surface-800 text-surface-600 dark:text-surface-400">{{ JSON.stringify(msg.data.payload, null, 2) }}</pre>
              </details>
            </div>

            <!-- Typing indicator -->
            <div v-if="typing" class="flex items-center gap-2 text-surface-400 pl-4">
              <span class="pi pi-spin pi-spinner text-sm"></span>
              <span class="text-xs font-medium">AI is thinking...</span>
            </div>
          </div>
        </template>
      </Card>
    </div>
  </div>
</template>

<style scoped>
.border-l-3 {
  border-left-width: 3px;
}
</style>
