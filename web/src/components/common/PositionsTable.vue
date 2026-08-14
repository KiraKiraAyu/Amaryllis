<script setup lang="ts">
import DataTable from "primevue/datatable"
import Column from "primevue/column"
import Button from "primevue/button"
import type { PositionPayload } from "@/types/trading"
import { formatDateTime, isLongSide, normalizeSide } from "@/utils/format"

withDefaults(
  defineProps<{
    positions: PositionPayload[]
    showTrader?: boolean
    traderName?: (id: string) => string
    closable?: boolean
    scrollHeight?: string
  }>(),
  {
    showTrader: false,
    closable: false,
    scrollHeight: "440px",
  },
)

const emit = defineEmits<{
  close: [traderId: string, symbol: string, side: string]
}>()

function fmt(value: number | null | undefined, digits = 2) {
  return (value ?? 0).toLocaleString("en-US", {
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  })
}

function signed(value: number) {
  return (value >= 0 ? "+" : "") + fmt(value, 2)
}

/**
 * Approximate initial margin = notional / leverage.
 * Accurate for isolated mode; cross-mode actual margin may differ
 * because the entire shared balance is at risk.
 */
function margin(data: PositionPayload): number {
  if (data.leverage <= 0) return 0
  return (data.entry_price * data.quantity) / data.leverage
}

/**
 * ROE% = unrealized PnL / initial margin × 100.
 * Uses the approximate initial margin above, so it is most accurate
 * for isolated positions and an estimate for cross positions.
 */
function roePct(data: PositionPayload): number | null {
  const m = margin(data)
  if (m <= 0) return null
  return (data.unrealized_pnl / m) * 100
}

function tpSlText(data: PositionPayload): string {
  const parts: string[] = []
  if (data.tp_price != null) parts.push(`TP ${fmt(data.tp_price, 2)}`)
  if (data.sl_price != null) parts.push(`SL ${fmt(data.sl_price, 2)}`)
  return parts.length > 0 ? parts.join(" / ") : "—"
}

function shortTraderId(id: string): string {
  if (id.length <= 12) return id
  return id.slice(0, 8) + "…"
}
</script>

<template>
  <div
    v-if="positions.length === 0"
    class="text-center py-12 border border-dashed border-surface-200 dark:border-surface-800 rounded-2xl bg-surface-50/50 dark:bg-surface-950/20"
  >
    <p class="text-sm text-surface-400 dark:text-surface-500">
      No active positions
    </p>
  </div>

  <div
    v-else
    class="overflow-hidden rounded-xl border border-surface-200 dark:border-surface-800"
  >
    <DataTable
      :value="positions"
      scrollable
      :scrollHeight="scrollHeight"
      class="text-xs w-full"
      stripedRows
    >
      <Column v-if="showTrader" field="trader_id" header="Trader">
        <template #body="{ data }">
          <span
            v-if="traderName"
            class="font-semibold text-surface-800 dark:text-surface-200"
          >
            {{ traderName(data.trader_id) }}
          </span>
          <span v-else class="font-mono text-surface-500" :title="data.trader_id">
            {{ shortTraderId(data.trader_id) }}
          </span>
        </template>
      </Column>

      <Column field="symbol" header="Symbol">
        <template #body="{ data }">
          <span class="font-mono font-bold text-primary">
            {{ data.symbol }}
          </span>
        </template>
      </Column>

      <Column field="side" header="Side">
        <template #body="{ data }">
          <span
            class="font-bold text-xs px-2 py-0.5 rounded-md"
            :class="
              isLongSide(data.side)
                ? 'bg-emerald-500/15 text-emerald-600 dark:text-emerald-400'
                : 'bg-rose-500/15 text-rose-600 dark:text-rose-400'
            "
          >
            {{ normalizeSide(data.side) }}
          </span>
        </template>
      </Column>

      <Column
        field="leverage"
        header="Lev."
        class="text-right"
        headerClass="justify-end"
      >
        <template #body="{ data }">
          <span class="font-mono text-surface-500">{{ data.leverage }}x</span>
        </template>
      </Column>

      <Column
        field="quantity"
        header="Size"
        class="text-right"
        headerClass="justify-end"
      >
        <template #body="{ data }">
          <span class="font-mono">{{ fmt(data.quantity, 4) }}</span>
        </template>
      </Column>

      <Column
        field="entry_price"
        header="Entry"
        class="text-right"
        headerClass="justify-end"
      >
        <template #body="{ data }">
          <span class="font-mono text-surface-600 dark:text-surface-400">{{
            fmt(data.entry_price, 2)
          }}</span>
        </template>
      </Column>

      <Column
        field="mark_price"
        header="Mark"
        class="text-right"
        headerClass="justify-end"
      >
        <template #body="{ data }">
          <span class="font-mono text-surface-600 dark:text-surface-400">{{
            fmt(data.mark_price, 2)
          }}</span>
        </template>
      </Column>

      <Column
        field="margin_mode"
        header="Margin"
        class="text-center"
        headerClass="justify-center"
      >
        <template #body="{ data }">
          <span
            class="text-xs px-1.5 py-0.5 rounded font-semibold capitalize"
            :class="
              data.margin_mode === 'isolated'
                ? 'bg-amber-500/10 text-amber-600 dark:text-amber-400'
                : 'bg-surface-100 dark:bg-surface-800 text-surface-500'
            "
          >
            {{ data.margin_mode || "cross" }}
          </span>
        </template>
      </Column>

      <Column
        field="margin"
        header="Margin $"
        class="text-right"
        headerClass="justify-end"
      >
        <template #body="{ data }">
          <span class="font-mono text-surface-500">{{ fmt(margin(data), 2) }}</span>
        </template>
      </Column>

      <Column
        field="liquidation_price"
        header="Liq."
        class="text-right"
        headerClass="justify-end"
      >
        <template #body="{ data }">
          <span
            class="font-mono"
            :class="
              data.liquidation_price > 0
                ? 'text-rose-500/80 dark:text-rose-400/80'
                : 'text-surface-400'
            "
          >
            {{ data.liquidation_price > 0 ? fmt(data.liquidation_price, 2) : "—" }}
          </span>
        </template>
      </Column>

      <Column
        field="unrealized_pnl"
        header="PNL"
        class="text-right"
        headerClass="justify-end"
      >
        <template #body="{ data }">
          <div class="flex flex-col items-end">
            <span
              class="font-mono font-bold"
              :class="
                data.unrealized_pnl >= 0
                  ? 'text-emerald-500'
                  : 'text-rose-500'
              "
            >
              {{ signed(data.unrealized_pnl) }}
            </span>
            <span
              v-if="roePct(data) != null"
              class="font-mono text-[10px]"
              :class="
                data.unrealized_pnl >= 0
                  ? 'text-emerald-500/70'
                  : 'text-rose-500/70'
              "
            >
              {{ data.unrealized_pnl >= 0 ? "+" : "" }}{{ fmt(roePct(data)!, 1) }}%
            </span>
          </div>
        </template>
      </Column>

      <Column
        field="tp_sl"
        header="TP / SL"
        class="text-right"
        headerClass="justify-end"
      >
        <template #body="{ data }">
          <span class="font-mono text-[10px] text-surface-500">
            {{ tpSlText(data) }}
          </span>
        </template>
      </Column>

      <Column field="opened_at" header="Opened At">
        <template #body="{ data }">
          <span class="text-surface-400 dark:text-surface-500">
            {{ formatDateTime(data.opened_at) }}
          </span>
        </template>
      </Column>

      <Column v-if="closable" class="w-12 text-center">
        <template #body="{ data }">
          <Button
            v-if="data.status === 'open'"
            icon="pi pi-times"
            severity="danger"
            text
            rounded
            @click="emit('close', data.trader_id, data.symbol, data.side)"
            title="Close position"
            class="h-8 w-8 cursor-pointer"
          />
        </template>
      </Column>
    </DataTable>
  </div>
</template>
