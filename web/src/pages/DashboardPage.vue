<script setup lang="ts">
import CreateTraderModal from "@/components/CreateTraderModal.vue"
import DashboardHeader from "@/components/dashboard/DashboardHeader.vue"
import DashboardStats from "@/components/dashboard/DashboardStats.vue"
import EquityCurvePanel from "@/components/dashboard/EquityCurvePanel.vue"
import LiveEventsPanel from "@/components/dashboard/LiveEventsPanel.vue"
import OpenPositionsPanel from "@/components/dashboard/OpenPositionsPanel.vue"
import TradersPanel from "@/components/dashboard/TradersPanel.vue"
import { useDashboardPage } from "@/composables/useDashboardPage"

const {
  activeChart,
  closePosition,
  equity,
  equityHistory,
  events,
  handleTraderCreated,
  initialLoadDone,
  loadAll,
  loading,
  positions,
  selectEquityTrader,
  showCreateTrader,
  startTrader,
  stopTrader,
  syncBalance,
  traderIdOptions,
  traderName,
  traders,
} = useDashboardPage()
</script>

<template>
  <div class="flex flex-col gap-6">
    <DashboardHeader @refresh="loadAll" />

    <DashboardStats
      :equity="equity"
      :traders="traders"
      :loading="loading"
      :initial-load-done="initialLoadDone"
    />

    <EquityCurvePanel
      :trader-ids="traderIdOptions"
      :active-trader-id="activeChart"
      :data="equityHistory"
      :trader-name="traderName"
      @select="selectEquityTrader"
    />

    <TradersPanel
        :traders="traders"
        :loading="loading"
        :initial-load-done="initialLoadDone"
        @create="showCreateTrader = true"
        @start="startTrader"
        @stop="stopTrader"
        @sync="syncBalance"
    />

    <OpenPositionsPanel
        :positions="positions"
        :trader-name="traderName"
        @close="closePosition"
    />

    <LiveEventsPanel :events="events" />
  </div>

  <CreateTraderModal
    v-if="showCreateTrader"
    @close="showCreateTrader = false"
    @created="handleTraderCreated"
  />
</template>
