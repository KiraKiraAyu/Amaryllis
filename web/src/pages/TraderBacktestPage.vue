<script setup lang="ts">
import { computed } from "vue"
import { useRoute, useRouter } from "vue-router"
import Button from "primevue/button"
import PageHeader from "@/components/layout/PageHeader.vue"
import BacktestConfigPanel from "@/components/backtest/BacktestConfigPanel.vue"
import BacktestLiveProgress from "@/components/backtest/BacktestLiveProgress.vue"
import BacktestRunsTable from "@/components/backtest/BacktestRunsTable.vue"
import { useBacktestPage } from "@/composables/useBacktestPage"

const route = useRoute()
const router = useRouter()

const traderId = computed(() => route.params.id as string)
const {
  cfg,
  liveProgress,
  loadingRuns,
  loadRuns,
  progressPct,
  running,
  runs,
  startRun,
  stopRun,
} = useBacktestPage(traderId.value)

function goBack() {
  router.push({ name: "trader-detail", params: { id: traderId.value } })
}
</script>

<template>
  <div class="flex flex-col gap-6">
    <PageHeader
      title="Backtest"
      description="Simulate this trader's strategy on historical data"
    >
      <template #actions>
        <div class="flex items-center gap-3">
          <Button
            label="Back"
            icon="pi pi-arrow-left"
            severity="secondary"
            variant="outlined"
            class="rounded-xl h-11 px-4 cursor-pointer"
            @click="goBack"
          />
          <Button
            @click="startRun"
            :disabled="running"
            :icon="running ? 'pi pi-spin pi-spinner' : 'pi pi-play'"
            :label="running ? 'Running...' : 'Start Backtest'"
            class="rounded-xl h-11 px-4 cursor-pointer"
          />
        </div>
      </template>
    </PageHeader>

    <BacktestConfigPanel v-model="cfg" />

    <BacktestRunsTable
      :runs="runs"
      :loading="loadingRuns"
      @refresh="loadRuns"
      @stop="stopRun"
    />

    <BacktestLiveProgress
      v-if="liveProgress"
      :progress="liveProgress"
      :progress-pct="progressPct"
    />
  </div>
</template>
