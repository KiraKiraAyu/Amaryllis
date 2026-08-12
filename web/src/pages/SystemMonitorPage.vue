<script setup lang="ts">
import AlertHistoryTable from "@/components/system-monitor/AlertHistoryTable.vue"
import RuntimeEventsPanel from "@/components/system-monitor/RuntimeEventsPanel.vue"
import SystemMetricsPanel from "@/components/system-monitor/SystemMetricsPanel.vue"
import SystemMonitorHeader from "@/components/system-monitor/SystemMonitorHeader.vue"
import { useSystemMonitorPage } from "@/composables/useSystemMonitorPage"

const {
  activeTrader,
  alerts,
  events,
  fmt,
  formatMetadata,
  loadAll,
  loading,
  metrics,
  parseAlerts,
  traders,
} = useSystemMonitorPage()
</script>

<template>
  <div class="flex flex-col gap-6 min-h-[calc(100vh-2rem)]">
    <SystemMonitorHeader
      v-model="activeTrader"
      :traders="traders"
      @refresh="loadAll"
    />

    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <SystemMetricsPanel :metrics="metrics" :loading="loading" :fmt="fmt">
        <AlertHistoryTable
          :alerts="alerts"
          :loading="loading"
          :parse-alerts="parseAlerts"
        />
      </SystemMetricsPanel>

      <RuntimeEventsPanel
        :events="events"
        :loading="loading"
        :format-metadata="formatMetadata"
      />
    </div>
  </div>
</template>
