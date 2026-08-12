<script setup lang="ts">
import { computed, ref } from "vue"
import { useRouter } from "vue-router"
import Button from "primevue/button"
import Dialog from "primevue/dialog"
import PageHeader from "@/components/layout/PageHeader.vue"
import SlideTransition from "@/components/SlideTransition.vue"
import TraderStats from "@/components/traders/TraderStats.vue"
import TraderManagementPanel from "@/components/traders/TraderManagementPanel.vue"
import CreateTraderForm from "@/components/traders/CreateTraderForm.vue"
import EditTraderForm from "@/components/traders/EditTraderForm.vue"
import EquityChart from "@/components/EquityChart.vue"
import { useTradersPage } from "@/composables/useTradersPage"
import { useCarouselTransition } from "@/composables/useCarouselTransition"
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
  startTrader,
  stats,
  stopTrader,
  syncBalance,
} = useTradersPage()

const router = useRouter()

function goToDetail(trader: TradersPageTrader) {
  router.push({ name: "trader-detail", params: { id: trader.id } })
}

function goToBacktest(id: string) {
  router.push({ name: "trader-backtest", params: { id } })
}

// Sub-page state: 0 = list, 1 = create, 2 = edit
const showCreate = ref(false)
const editingTrader = ref<TradersPageTrader | null>(null)

const step = computed(() => {
  if (editingTrader.value) return 2
  if (showCreate.value) return 1
  return 0
})

const { direction } = useCarouselTransition(step)

const showDialog = computed({
  get: () => selectedTrader.value !== null,
  set: (val) => {
    if (!val) selectedTrader.value = null
  },
})

function handleCreated() {
  showCreate.value = false
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
            <span
              class="w-2 h-2 rounded-full bg-emerald-500 animate-pulse"
            ></span>
            <span class="text-xs text-surface-500 font-medium">{{
              lastUpdated
            }}</span>
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
            @click="showCreate = true"
          />
        </div>
      </template>
    </PageHeader>

    <div class="relative w-full overflow-hidden">
      <SlideTransition :direction="direction">
        <!-- View 0: Trader List -->
        <div v-if="step === 0" key="list" class="w-full flex flex-col gap-6">
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
            @select="goToDetail"
            @start="startTrader"
            @stop="stopTrader"
            @sync="syncBalance"
            @edit="editingTrader = $event"
            @delete="deleteTrader"
            @backtest="goToBacktest"
          />
        </div>

        <!-- View 1: Create Trader -->
        <div v-else-if="step === 1" key="create" class="w-full">
          <CreateTraderForm
            @close="showCreate = false"
            @created="handleCreated"
          />
        </div>

        <!-- View 2: Edit Trader -->
        <div v-else-if="step === 2" key="edit" class="w-full">
          <EditTraderForm
            :trader="editingTrader!"
            @close="editingTrader = null"
            @updated="handleUpdated"
          />
        </div>
      </SlideTransition>
    </div>

    <Dialog
      v-model:visible="showDialog"
      modal
      :header="
        selectedTrader
          ? `${selectedTrader.name || selectedTrader.id} - Equity Curve`
          : 'Equity Curve'
      "
      :style="{ width: '50rem' }"
    >
      <div class="pt-2">
        <EquityChart :data="selectedEquity" :height="320" />
      </div>
    </Dialog>
  </div>
</template>
