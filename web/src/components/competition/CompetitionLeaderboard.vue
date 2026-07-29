<script setup lang="ts">
import InputText from "primevue/inputtext"
import IconField from "primevue/iconfield"
import InputIcon from "primevue/inputicon"
import DataTable from "primevue/datatable"
import Column from "primevue/column"
import type { CompetitionTrader } from "@/types/competition-ui"

defineProps<{
  traders: CompetitionTrader[]
  loading: boolean
  avatarStyle: (id: string) => string
  fmt: (value: number) => string
  returnPct: (trader: CompetitionTrader) => number
}>()

const search = defineModel<string>({ required: true })

const emit = defineEmits<{
  select: [trader: CompetitionTrader]
}>()
</script>

<template>
  <div class="bg-surface-0 dark:bg-surface-900 border border-surface-200 dark:border-surface-800 rounded-2xl p-6">
    <div class="flex items-center justify-between mb-4 flex-wrap gap-4">
      <div>
        <h2 class="font-bold text-lg text-surface-900 dark:text-white">Full Rankings</h2>
      </div>
      <IconField>
        <InputIcon class="pi pi-search" />
        <InputText
          v-model="search"
          size="small"
          placeholder="Search..."
          class="rounded-xl w-64"
        />
      </IconField>
    </div>

    <DataTable
      :value="traders"
      :loading="loading"
      responsiveLayout="scroll"
      @row-click="emit('select', $event.data)"
      :pt="{
        root: { class: 'text-sm' },
        headerRow: { class: 'bg-surface-50 dark:bg-surface-900' },
        row: { class: 'cursor-pointer hover:bg-surface-100 dark:hover:bg-surface-800 transition-colors' }
      }"
    >
      <template #empty>
        <div class="text-center py-12 text-surface-500">No active traders in competition</div>
      </template>
      <template #loading>
        <div class="text-center py-12 text-surface-500">Loading...</div>
      </template>

      <Column header="#" style="width: 5%">
        <template #body="{ index }">
          <span class="font-bold text-lg" :class="index < 3 ? 'text-primary-500' : 'text-surface-500'">
            {{ index < 3 ? ["🥇", "🥈", "🥉"][index] : index + 1 }}
          </span>
        </template>
      </Column>

      <Column header="Trader" style="width: 25%">
        <template #body="{ data }">
          <div class="flex items-center gap-3">
            <div
              class="w-8 h-8 rounded-full flex items-center justify-center text-xs font-bold shrink-0 text-white shadow-sm"
              :style="avatarStyle(data.trader_id)"
            >
              {{ (data.trader_name || data.trader_id).charAt(0).toUpperCase() }}
            </div>
            <div>
              <p class="font-semibold text-surface-900 dark:text-surface-100">
                {{ data.trader_name || data.trader_id.slice(0, 16) }}
              </p>
              <p class="text-xs text-surface-500 font-medium tracking-wide">
                {{ data.exchange.toUpperCase() }}
              </p>
            </div>
          </div>
        </template>
      </Column>

      <Column field="ai_model" header="Model" style="width: 15%">
        <template #body="{ data }">
          <span class="text-xs px-2 py-1 rounded bg-surface-100 dark:bg-surface-800 text-surface-700 dark:text-surface-300 font-medium">
            {{ data.ai_model }}
          </span>
        </template>
      </Column>

      <Column header="Equity" align="right" style="width: 15%">
        <template #body="{ data }">
          <span class="font-mono font-medium text-surface-900 dark:text-surface-100">
            ${{ fmt(data.total_equity) }}
          </span>
        </template>
      </Column>

      <Column header="Return" align="right" style="width: 15%">
        <template #body="{ data }">
          <span
            class="font-mono font-bold"
            :class="returnPct(data) >= 0 ? 'text-emerald-500 dark:text-emerald-400' : 'text-rose-500 dark:text-rose-400'"
          >
            {{ (returnPct(data) >= 0 ? "+" : "") + returnPct(data).toFixed(2) }}%
          </span>
        </template>
      </Column>

      <Column field="position_count" header="Positions" align="right" style="width: 10%">
        <template #body="{ data }">
          <span class="font-mono text-surface-700 dark:text-surface-300">{{ data.position_count }}</span>
        </template>
      </Column>

      <Column header="Margin" align="right" style="width: 10%">
        <template #body="{ data }">
          <span class="font-mono text-surface-700 dark:text-surface-300">
            {{ data.margin_used_pct != null ? data.margin_used_pct.toFixed(1) + "%" : "-" }}
          </span>
        </template>
      </Column>

      <Column header="Status" align="right" style="width: 5%">
        <template #body="{ data }">
          <span
            class="text-xs font-bold uppercase tracking-wide px-1.5 py-0.5 rounded"
            :class="data.is_running ? 'bg-emerald-100 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400' : 'bg-surface-100 text-surface-600 dark:bg-surface-800 dark:text-surface-400'"
          >
            {{ data.is_running ? "Live" : "Stopped" }}
          </span>
        </template>
      </Column>
    </DataTable>
  </div>
</template>
