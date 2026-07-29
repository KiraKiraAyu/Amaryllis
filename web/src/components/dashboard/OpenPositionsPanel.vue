<script setup lang="ts">
import Card from "primevue/card"
import Button from "primevue/button"
import DataTable from "primevue/datatable"
import Column from "primevue/column"
import type { DashboardPosition } from "@/types/dashboard-ui"
import { formatDateTime } from "@/utils/format"

defineProps<{
  positions: DashboardPosition[]
  traderName: (id: string) => string
}>()

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
</script>

<template>
  <Card class="border border-surface-200 dark:border-surface-800 bg-surface-0 dark:bg-surface-900 shadow-none!">
    <template #content>
      <div class="flex items-center justify-between mb-4">
        <h2 class="font-bold text-lg text-surface-900 dark:text-white">Open Positions</h2>
        <span class="text-xs font-semibold px-2.5 py-1 bg-surface-100 dark:bg-surface-800 rounded-lg text-surface-600 dark:text-surface-400">
          {{ positions.length }} active
        </span>
      </div>

      <div v-if="positions.length === 0" class="text-center py-12 border border-dashed border-surface-200 dark:border-surface-800 rounded-2xl bg-surface-50/50 dark:bg-surface-950/20">
        <p class="text-sm text-surface-400 dark:text-surface-500">No active positions</p>
      </div>

      <div v-else class="overflow-hidden rounded-xl border border-surface-200 dark:border-surface-800">
        <DataTable
          :value="positions"
          scrollable
          scrollHeight="440px"
          class="text-xs w-full"
          stripedRows
        >
          <Column field="trader_id" header="Trader">
            <template #body="{ data }">
              <span class="font-semibold text-surface-800 dark:text-surface-200">
                {{ traderName(data.trader_id) }}
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
                class="font-bold uppercase text-xs px-2 py-0.5 rounded-md"
                :class="
                  data.side.toLowerCase() === 'buy' || data.side.toLowerCase() === 'long'
                    ? 'bg-emerald-500/15 text-emerald-600 dark:text-emerald-400'
                    : 'bg-rose-500/15 text-rose-600 dark:text-rose-400'
                "
              >
                {{ data.side }}
              </span>
            </template>
          </Column>

          <Column field="quantity" header="Qty" class="text-right" headerClass="justify-end">
            <template #body="{ data }">
              <span class="font-mono">{{ fmt(data.quantity, 4) }}</span>
            </template>
          </Column>

          <Column field="entry_price" header="Entry" class="text-right" headerClass="justify-end">
            <template #body="{ data }">
              <span class="font-mono text-surface-600 dark:text-surface-400">{{ fmt(data.entry_price, 2) }}</span>
            </template>
          </Column>

          <Column field="mark_price" header="Mark" class="text-right" headerClass="justify-end">
            <template #body="{ data }">
              <span class="font-mono text-surface-600 dark:text-surface-400">{{ fmt(data.mark_price, 2) }}</span>
            </template>
          </Column>

          <Column field="leverage" header="Lev." class="text-right" headerClass="justify-end">
            <template #body="{ data }">
              <span class="font-mono text-surface-500">{{ data.leverage }}x</span>
            </template>
          </Column>

          <Column field="unrealized_pnl" header="Unrealized PnL" class="text-right" headerClass="justify-end">
            <template #body="{ data }">
              <span
                class="font-mono font-bold"
                :class="data.unrealized_pnl >= 0 ? 'text-emerald-500' : 'text-rose-500'"
              >
                {{ signed(data.unrealized_pnl) }}
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

          <Column class="w-12 text-center">
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
  </Card>
</template>
