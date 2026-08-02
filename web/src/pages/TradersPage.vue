<script setup lang="ts">
import { computed, ref } from "vue"
import Button from "primevue/button"
import Dialog from "primevue/dialog"
import PageHeader from "@/components/layout/PageHeader.vue"
import TraderStats from "@/components/traders/TraderStats.vue"
import TraderManagementPanel from "@/components/traders/TraderManagementPanel.vue"
import EquityChart from "@/components/EquityChart.vue"
import CreateTraderModal from "@/components/CreateTraderModal.vue"
import EditTraderModal from "@/components/EditTraderModal.vue"
import { useTradersPage } from "@/composables/useTradersPage"
import type { TradersPageTrader } from "@/composables/useTradersPage"

const {
  avatarStyle,
  deleteTrader,
  filtered,
  fmt,
  lastUpdated,
  load,
  loading,
  returnPct,
  search,
  selectedEquity,
  selectedTrader,
  showDetail,
  startTrader,
  stats,
  stopTrader,
  syncBalance,
} = useTradersPage()

const showCreateModal = ref(false)
const editingTrader = ref<TradersPageTrader | null>(null)

const showDialog = computed({
  get: () => selectedTrader.value !== null,
  set: (val) => {
    if (!val) selectedTrader.value = null
  },
})

function handleCreated() {
  showCreateModal.value = false
  load()
}

function handleUpdated() {
  editingTrader.value = null
  load()
}
</script>

<template>
  <div class="flex flex-col gap-6">
    <PageHeader
      title="Traders"
      description="Manage your AI traders - create, edit, start and stop"
    >
      <template #actions>
        <div class="flex items-center gap-3">
          <div class="flex items-center gap-2">
            <span class="w-2 h-2 rounded-full bg-emerald-500 animate-pulse"></span>
            <span class="text-xs text-surface-500 font-medium">{{ lastUpdated }}</span>
          </div>
          <Button
            label="Refresh"
            icon="pi pi-refresh"
            severity="secondary"
            variant="outlined"
            class="rounded-xl h-11 px-4 cursor-pointer"
            @click="load"
          />
          <Button
            label="New Trader"
            icon="pi pi-plus"
            class="rounded-xl h-11 px-4 cursor-pointer"
            @click="showCreateModal = true"
          />
        </div>
      </template>
    </PageHeader>

    <TraderStats
      :total="stats.total"
      :running="stats.running"
      :stopped="stats.stopped"
      :total-pnl="stats.totalPnl"
      :loading="loading"
    />

    <TraderManagementPanel
      v-model="search"
      :traders="filtered"
      :loading="loading"
      :avatar-style="avatarStyle"
      :fmt="fmt"
      :return-pct="returnPct"
      @select="showDetail"
      @start="startTrader"
      @stop="stopTrader"
      @sync="syncBalance"
      @edit="editingTrader = $event"
      @delete="deleteTrader"
    />

    <Dialog
      v-model:visible="showDialog"
      modal
      :header="selectedTrader ? `${selectedTrader.name || selectedTrader.id} - Equity Curve` : 'Equity Curve'"
      :style="{ width: '50rem' }"
    >
      <div class="pt-2">
        <EquityChart :data="selectedEquity" :height="320" />
      </div>
    </Dialog>

    <CreateTraderModal
      v-if="showCreateModal"
      @close="showCreateModal = false"
      @created="handleCreated"
    />

    <EditTraderModal
      v-if="editingTrader"
      :trader="editingTrader"
      @close="editingTrader = null"
      @updated="handleUpdated"
    />
  </div>
</template>
