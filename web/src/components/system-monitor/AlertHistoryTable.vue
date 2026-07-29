<script setup lang="ts">
import DataTable from "primevue/datatable"
import Column from "primevue/column"
import type { RuntimeAlertHistoryItemPayload, RuntimeAlertItemPayload } from "@/types/trading"
import { formatDateTime } from "@/utils/format"

defineProps<{
  alerts: RuntimeAlertHistoryItemPayload[]
  loading: boolean
  parseAlerts: (value: RuntimeAlertItemPayload[]) => string
}>()
</script>

<template>
  <div class="flex-1 overflow-hidden flex flex-col">
    <div class="flex items-center justify-between mb-4">
      <h2 class="font-bold text-sm text-surface-900 dark:text-white">Alert History</h2>
      <span class="text-surface-500 text-sm">{{ alerts.length }} total</span>
    </div>
    
    <div class="flex-1 overflow-hidden border border-surface-200 dark:border-surface-800 rounded-xl bg-surface-0 dark:bg-surface-900">
      <DataTable
        :value="alerts"
        :loading="loading"
        scrollable
        scrollHeight="300px"
        :pt="{
          root: { class: 'text-xs' },
          headerRow: { class: 'bg-surface-50 dark:bg-surface-900/50' }
        }"
      >
        <template #empty>
          <div class="text-center py-10 text-surface-500">No alerts logged in the selected window.</div>
        </template>
        <template #loading>
          <div class="text-center py-10 text-surface-500">Loading...</div>
        </template>

        <Column field="created_at" header="Time" style="width: 25%">
          <template #body="{ data }">
            <span class="text-surface-500">{{ formatDateTime(data.created_at) }}</span>
          </template>
        </Column>

        <Column field="severity" header="Severity" style="width: 15%">
          <template #body="{ data }">
            <span 
              class="font-medium px-2 py-0.5 rounded text-xs uppercase tracking-wide"
              :class="{
                'bg-rose-100 text-rose-700 dark:bg-rose-900/40 dark:text-rose-400': data.severity === 'error' || data.severity === 'critical',
                'bg-amber-100 text-amber-700 dark:bg-amber-900/40 dark:text-amber-400': data.severity === 'warning',
                'bg-blue-100 text-blue-700 dark:bg-blue-900/40 dark:text-blue-400': data.severity === 'info'
              }"
            >
              {{ data.severity }}
            </span>
          </template>
        </Column>

        <Column field="alerts" header="Details" style="width: 60%">
          <template #body="{ data }">
            <span class="text-surface-900 dark:text-surface-100">
              Triggers: {{ parseAlerts(data.alerts) }}
            </span>
          </template>
        </Column>
      </DataTable>
    </div>
  </div>
</template>
