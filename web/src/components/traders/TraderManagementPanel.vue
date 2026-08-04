<script setup lang="ts">
import Card from "primevue/card"
import Button from "primevue/button"
import InputText from "primevue/inputtext"
import IconField from "primevue/iconfield"
import InputIcon from "primevue/inputicon"
import DataTable from "primevue/datatable"
import Column from "primevue/column"
import type { TradersPageTrader } from "@/composables/useTradersPage"

defineProps<{
  traders: TradersPageTrader[]
  loading: boolean
  avatarStyle: (id: string) => string
  fmt: (value: number) => string
  returnPct: (trader: TradersPageTrader) => number
}>()

const search = defineModel<string>({ required: true })

const emit = defineEmits<{
  select: [trader: TradersPageTrader]
  start: [id: string]
  stop: [id: string]
  sync: [id: string]
  edit: [trader: TradersPageTrader]
  delete: [id: string]
}>()
</script>

<template>
  <Card
    class="border border-surface-200 dark:border-surface-800 bg-surface-0 dark:bg-surface-900 shadow-none!"
  >
    <template #content>
      <div class="flex items-center justify-between mb-4 flex-wrap gap-4">
        <h2 class="font-bold text-lg text-surface-900 dark:text-white">
          All Traders
        </h2>
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
          root: { class: 'text-sm traders-table' },
          headerRow: { class: 'bg-surface-50 dark:bg-surface-900' },
        }"
      >
        <template #empty>
          <div
            class="text-center py-12 border border-dashed border-surface-200 dark:border-surface-800 rounded-2xl bg-surface-50/50 dark:bg-surface-950/20"
          >
            <p class="text-sm text-surface-400 dark:text-surface-500">
              No traders yet. Click "New Trader" to create one.
            </p>
          </div>
        </template>
        <template #loading>
          <div class="text-center py-12 text-surface-500">
            <span class="pi pi-spin pi-spinner mr-2"></span>
            Loading traders...
          </div>
        </template>

        <Column header="Trader" style="width: 20%">
          <template #body="{ data }">
            <div class="flex items-center gap-3">
              <div
                class="w-8 h-8 rounded-full flex items-center justify-center text-xs font-bold shrink-0 text-white shadow-sm"
                :style="avatarStyle(data.id)"
              >
                {{ (data.name || data.id).charAt(0).toUpperCase() }}
              </div>
              <div class="min-w-0">
                <p
                  class="font-semibold text-surface-900 dark:text-surface-100 truncate"
                >
                  {{ data.name || data.id.slice(0, 16) }}
                </p>
                <p class="text-xs text-surface-500 font-medium tracking-wide">
                  {{ data.exchange_id || "Paper" }}
                </p>
              </div>
            </div>
          </template>
        </Column>

        <Column field="ai_model_id" header="Model" style="width: 13%">
          <template #body="{ data }">
            <span
              class="text-xs px-2 py-1 rounded bg-surface-100 dark:bg-surface-800 text-surface-700 dark:text-surface-300 font-medium"
            >
              {{ data.ai_model_id }}
            </span>
          </template>
        </Column>

        <Column header="Unrealized PnL" align="right" style="width: 12%">
          <template #body="{ data }">
            <span
              class="font-mono font-medium"
              :class="
                (data.unrealized_pnl || 0) >= 0
                  ? 'text-emerald-500 dark:text-emerald-400'
                  : 'text-rose-500 dark:text-rose-400'
              "
            >
              {{ (data.unrealized_pnl || 0) >= 0 ? "+" : "" }}${{
                fmt(data.unrealized_pnl || 0)
              }}
            </span>
          </template>
        </Column>

        <Column header="Realized PnL" align="right" style="width: 12%">
          <template #body="{ data }">
            <span
              class="font-mono font-medium"
              :class="
                (data.realized_pnl || 0) >= 0
                  ? 'text-emerald-500 dark:text-emerald-400'
                  : 'text-rose-500 dark:text-rose-400'
              "
            >
              {{ (data.realized_pnl || 0) >= 0 ? "+" : "" }}${{
                fmt(data.realized_pnl || 0)
              }}
            </span>
          </template>
        </Column>

        <Column header="Return" align="right" style="width: 10%">
          <template #body="{ data }">
            <span
              class="font-mono font-bold"
              :class="
                returnPct(data) >= 0
                  ? 'text-emerald-500 dark:text-emerald-400'
                  : 'text-rose-500 dark:text-rose-400'
              "
            >
              {{
                (returnPct(data) >= 0 ? "+" : "") + returnPct(data).toFixed(2)
              }}%
            </span>
          </template>
        </Column>

        <Column
          field="position_count"
          header="Positions"
          align="right"
          style="width: 8%"
        >
          <template #body="{ data }">
            <span class="font-mono text-surface-700 dark:text-surface-300">{{
              data.position_count
            }}</span>
          </template>
        </Column>

        <Column header="Status" align="center" style="width: 8%">
          <template #body="{ data }">
            <div class="flex items-center justify-center gap-1.5">
              <span class="relative flex h-2.5 w-2.5">
                <span
                  v-if="data.is_running"
                  class="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"
                ></span>
                <span
                  class="relative inline-flex rounded-full h-2.5 w-2.5"
                  :class="
                    data.is_running
                      ? 'bg-emerald-500'
                      : 'bg-surface-300 dark:bg-surface-600'
                  "
                ></span>
              </span>
              <span
                class="text-xs font-bold uppercase tracking-wider"
                :class="
                  data.is_running ? 'text-emerald-500' : 'text-surface-500'
                "
              >
                {{ data.is_running ? "Live" : "Stopped" }}
              </span>
            </div>
          </template>
        </Column>

        <Column header="Actions" align="center" style="width: 17%">
          <template #body="{ data }">
            <div class="flex items-center justify-center gap-1.5">
              <!-- Start / Stop -->
              <Button
                v-if="!data.is_running"
                icon="pi pi-play"
                severity="success"
                rounded
                size="small"
                @click.stop="emit('start', data.id)"
                title="Start trader"
                class="h-8 w-8 cursor-pointer bg-emerald-500! border-emerald-500! hover:bg-emerald-600! hover:border-emerald-600! text-white!"
              />
              <Button
                v-else
                icon="pi pi-stop"
                severity="danger"
                rounded
                size="small"
                @click.stop="emit('stop', data.id)"
                title="Stop trader"
                class="h-8 w-8 cursor-pointer bg-rose-500! border-rose-500! hover:bg-rose-600! hover:border-rose-600! text-white!"
              />

              <!-- Sync balance -->
              <Button
                icon="pi pi-refresh"
                severity="secondary"
                text
                rounded
                size="small"
                @click.stop="emit('sync', data.id)"
                title="Sync balance"
                class="h-8 w-8 cursor-pointer"
              />

              <!-- Edit -->
              <Button
                icon="pi pi-pencil"
                severity="secondary"
                text
                rounded
                size="small"
                @click.stop="emit('edit', data)"
                title="Edit trader"
                class="h-8 w-8 cursor-pointer"
              />

              <!-- Delete -->
              <Button
                icon="pi pi-trash"
                severity="danger"
                text
                rounded
                size="small"
                @click.stop="emit('delete', data.id)"
                title="Delete trader"
                class="h-8 w-8 cursor-pointer text-rose-500!"
              />
            </div>
          </template>
        </Column>
      </DataTable>
    </template>
  </Card>
</template>

<style>
/* Non-scoped: PrimeVue v4 DataTable rows use high-specificity selectors
   that override scoped :deep(). We target rows via a root-level class
   on the DataTable itself, using PrimeVue semantic variables from style.css. */

.traders-table .p-datatable-tbody > tr {
  cursor: pointer;
  transition: background-color 0.2s ease;
}

.traders-table .p-datatable-tbody > tr:hover {
  background-color: var(--p-primary-50) !important;
}

.traders-table .p-datatable-tbody > tr.p-row-odd:hover {
  background-color: var(--p-primary-50) !important;
}

.dark .traders-table .p-datatable-tbody > tr:hover {
  background-color: var(--p-primary-900) !important;
}

.dark .traders-table .p-datatable-tbody > tr.p-row-odd:hover {
  background-color: var(--p-primary-900) !important;
}
</style>
