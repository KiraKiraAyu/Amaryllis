<script setup lang="ts">
import Card from "primevue/card"
import Button from "primevue/button"
import DataTable from "primevue/datatable"
import Column from "primevue/column"
import type { BacktestRun } from "@/types/backtest-ui"
import { formatDate } from "@/utils/format"

defineProps<{
  runs: BacktestRun[]
  loading: boolean
}>()

const emit = defineEmits<{
  refresh: []
  stop: [runId: string]
}>()

function returnPct(run: BacktestRun) {
  const equity = run.summary?.equity_last ?? 0
  const initial = run.summary?.initial_balance ?? 1000
  return initial > 0 ? ((equity - initial) / initial) * 100 : 0
}
</script>

<template>
  <Card class="border border-surface-200 dark:border-surface-800 bg-surface-0 dark:bg-surface-900 shadow-none!">
    <template #content>
      <div class="flex items-center justify-between mb-4">
        <h2 class="font-bold text-lg text-surface-900 dark:text-white">Backtest Runs</h2>
        <Button
          icon="pi pi-refresh"
          label="Refresh"
          severity="secondary"
          variant="outlined"
          @click="emit('refresh')"
          class="rounded-xl h-11 px-4 cursor-pointer"
        />
      </div>

      <div v-if="runs.length === 0 && !loading" class="text-center py-12 border border-dashed border-surface-200 dark:border-surface-800 rounded-2xl bg-surface-50/50 dark:bg-surface-950/20">
        <p class="text-sm text-surface-400 dark:text-surface-500">No backtest runs found. Start one above.</p>
      </div>

      <div v-else class="overflow-hidden rounded-xl border border-surface-200 dark:border-surface-800">
        <DataTable
          :value="runs"
          scrollable
          class="text-xs w-full"
          stripedRows
        >
          <Column field="run_id" header="Run ID">
            <template #body="{ data }">
              <span class="font-mono text-surface-600 dark:text-surface-400">
                {{ data.run_id.slice(0, 8) }}
              </span>
            </template>
          </Column>

          <Column field="state" header="State">
            <template #body="{ data }">
              <span
                class="text-xs font-black uppercase tracking-wider px-2 py-0.5 rounded-md"
                :class="
                  data.state === 'running'
                    ? 'bg-amber-500/15 text-amber-600 dark:text-amber-400 animate-pulse'
                    : data.state === 'completed' || data.state === 'success'
                    ? 'bg-emerald-500/15 text-emerald-600 dark:text-emerald-400'
                    : 'bg-surface-100 text-surface-500 dark:bg-surface-800'
                "
              >
                {{ data.state }}
              </span>
            </template>
          </Column>

          <Column field="summary.equity_last" header="Equity" class="text-right" headerClass="justify-end">
            <template #body="{ data }">
              <span class="font-mono">
                ${{ (data.summary?.equity_last ?? 0).toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 }) }}
              </span>
            </template>
          </Column>

          <Column header="Return" class="text-right" headerClass="justify-end">
            <template #body="{ data }">
              <span
                class="font-mono font-bold"
                :class="returnPct(data) >= 0 ? 'text-emerald-500' : 'text-rose-500'"
              >
                {{ (returnPct(data) >= 0 ? "+" : "") + returnPct(data).toFixed(2) }}%
              </span>
            </template>
          </Column>

          <Column field="summary.max_drawdown_pct" header="Max DD" class="text-right" headerClass="justify-end">
            <template #body="{ data }">
              <span class="font-mono text-rose-450 dark:text-rose-400">{{ (data.summary?.max_drawdown_pct ?? 0).toFixed(2) }}%</span>
            </template>
          </Column>

          <Column field="created_at" header="Created">
            <template #body="{ data }">
              <span class="text-surface-400 dark:text-surface-500">
                {{ formatDate(data.created_at) }}
              </span>
            </template>
          </Column>

          <Column class="w-20 text-right">
            <template #body="{ data }">
              <Button
                v-if="data.state === 'running'"
                label="Stop"
                severity="danger"
                size="small"
                text
                @click="emit('stop', data.run_id)"
                class="rounded-lg px-2.5 h-8 font-semibold cursor-pointer"
              />
            </template>
          </Column>
        </DataTable>
      </div>
    </template>
  </Card>
</template>
