<script setup lang="ts">
import { computed, onMounted, watch } from "vue"
import { useRoute, useRouter } from "vue-router"
import Button from "primevue/button"
import ProgressSpinner from "primevue/progressspinner"
import PageHeader from "@/components/layout/PageHeader.vue"
import TraderFeed from "@/components/traders/feed/TraderFeed.vue"
import ScanCountdown from "@/components/traders/ScanCountdown.vue"
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
  filteredFeed,
  typing,
  showSystemOps,
  nextScanAt,
  loadAll,
  startTrader,
  stopTrader,
} = useTraderDetail(traderId)

// Reload when trader ID changes
watch(traderId, () => loadAll(), { immediate: false })

onMounted(loadAll)

function backToList() {
  router.push({ name: "traders" })
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
      :description="
        trader
          ? `Model: ${trader.ai_model_id} · Exchange: ${trader.exchange_id}`
          : 'Loading...'
      "
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
      <div
        class="flex items-center gap-6 p-4 rounded-2xl bg-surface-50 dark:bg-surface-950 border border-surface-200 dark:border-surface-800 flex-wrap"
      >
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
          <span
            class="text-sm font-bold"
            :class="trader.is_running ? 'text-emerald-500' : 'text-surface-500'"
          >
            {{ trader.is_running ? "Running" : "Stopped" }}
          </span>
        </div>

        <div class="w-px h-6 bg-surface-200 dark:bg-surface-800"></div>

        <!-- Balance -->
        <div class="flex flex-col">
          <span
            class="text-xs text-surface-400 font-bold uppercase tracking-wider"
            >Balance</span
          >
          <span
            class="text-sm font-bold font-mono text-surface-900 dark:text-white"
          >
            ${{ fmtUsd(account?.total_balance ?? trader.initial_balance) }}
          </span>
        </div>

        <!-- Unrealized PnL -->
        <div class="flex flex-col" v-if="account">
          <span
            class="text-xs text-surface-400 font-bold uppercase tracking-wider"
            >Unrealized PnL</span
          >
          <span
            class="text-sm font-bold font-mono"
            :class="pnlClass(account.unrealized_pnl)"
          >
            {{ account.unrealized_pnl >= 0 ? "+" : "" }}${{
              fmtUsd(account.unrealized_pnl)
            }}
          </span>
        </div>

        <!-- Realized PnL -->
        <div class="flex flex-col" v-if="account">
          <span
            class="text-xs text-surface-400 font-bold uppercase tracking-wider"
            >Realized PnL</span
          >
          <span
            class="text-sm font-bold font-mono"
            :class="pnlClass(account.realized_pnl)"
          >
            {{ account.realized_pnl >= 0 ? "+" : "" }}${{
              fmtUsd(account.realized_pnl)
            }}
          </span>
        </div>

        <!-- Open Positions -->
        <div class="flex flex-col">
          <span
            class="text-xs text-surface-400 font-bold uppercase tracking-wider"
            >Positions</span
          >
          <span
            class="text-sm font-bold font-mono text-surface-900 dark:text-white"
            >{{ positions.length }}</span
          >
        </div>

        <!-- Budget -->
        <div class="flex flex-col">
          <span
            class="text-xs text-surface-400 font-bold uppercase tracking-wider"
            >Budget</span
          >
          <span
            class="text-sm font-bold font-mono text-surface-900 dark:text-white"
            >${{ fmtUsd(trader.initial_balance) }}</span
          >
        </div>

        <!-- Scan Interval -->
        <div class="flex flex-col">
          <span
            class="text-xs text-surface-400 font-bold uppercase tracking-wider"
            >Scan Interval</span
          >
          <span
            class="text-sm font-bold font-mono text-surface-900 dark:text-white"
            >{{ trader.scan_interval_minutes }}m</span
          >
        </div>

        <!-- Next Scan Countdown -->
        <ScanCountdown
          :next-scan-at="nextScanAt"
          :is-running="trader.is_running"
        />
      </div>

      <!-- Open positions summary -->
      <div v-if="positions.length > 0" class="flex flex-wrap gap-2">
        <div
          v-for="pos in positions"
          :key="pos.id"
          class="flex items-center gap-2 px-3 py-1.5 rounded-xl text-xs font-mono border"
          :class="
            pos.side === 'LONG'
              ? 'border-emerald-200 dark:border-emerald-800 bg-emerald-50 dark:bg-emerald-950/20'
              : 'border-rose-200 dark:border-rose-800 bg-rose-50 dark:bg-rose-950/20'
          "
        >
          <span
            class="font-bold"
            :class="
              pos.side === 'LONG'
                ? 'text-emerald-600 dark:text-emerald-400'
                : 'text-rose-600 dark:text-rose-400'
            "
          >
            {{ pos.side }}
          </span>
          <span class="font-bold text-surface-900 dark:text-white">{{
            pos.symbol
          }}</span>
          <span class="text-surface-500">{{ pos.quantity }}</span>
          <span class="text-surface-400"
            >@ ${{ pos.entry_price.toFixed(2) }}</span
          >
          <span :class="pnlClass(pos.unrealized_pnl)">
            {{ pos.unrealized_pnl >= 0 ? "+" : "" }}${{
              pos.unrealized_pnl.toFixed(2)
            }}
          </span>
        </div>
      </div>

      <!-- Chat feed -->
      <TraderFeed
        :messages="feed"
        :filtered-messages="filteredFeed"
        :typing="typing"
        :show-system-ops="showSystemOps"
        @update:show-system-ops="showSystemOps = $event"
      />
    </div>
  </div>
</template>
