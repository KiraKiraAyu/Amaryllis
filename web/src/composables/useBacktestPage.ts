import { onMounted, ref, watch } from "vue"
import {
  getBacktestRunsApi,
  startBacktestApi,
  stopBacktestApi,
} from "@/api/backtest"
import { useRealtimeStore } from "@/stores/realtime"
import type {
  BacktestConfig,
  BacktestLiveProgress,
  BacktestRun,
} from "@/types/backtest-ui"

export function useBacktestPage(traderId: string) {
  const realtime = useRealtimeStore()
  const runs = ref<BacktestRun[]>([])
  const loadingRuns = ref(false)
  const running = ref(false)
  const liveProgress = ref<BacktestLiveProgress | null>(null)
  const progressPct = ref(0)
  const cfg = ref<BacktestConfig>({
    interval: "5m",
    startDate: new Date(Date.now() - 7 * 24 * 3600 * 1000)
      .toISOString()
      .slice(0, 10),
    endDate: new Date().toISOString().slice(0, 10),
    initial_balance: 1000,
  })

  async function loadRuns() {
    loadingRuns.value = true
    try {
      const data = await getBacktestRunsApi()
      runs.value = data.runs
    } finally {
      loadingRuns.value = false
    }
  }

  async function startRun() {
    running.value = true
    try {
      const startTs = Math.floor(new Date(cfg.value.startDate).getTime() / 1000)
      const endTs = Math.floor(new Date(cfg.value.endDate).getTime() / 1000)
      await startBacktestApi({
        trader_id: traderId,
        interval: cfg.value.interval,
        start_ts: startTs,
        end_ts: endTs,
        initial_balance: cfg.value.initial_balance,
      })
      await loadRuns()
    } finally {
      running.value = false
    }
  }

  async function stopRun(id: string) {
    await stopBacktestApi({ run_id: id })
    await loadRuns()
  }

  watch(
    () => realtime.lastEvent,
    (event) => {
      if (event?.type !== "backtest_progress") return
      const barIndex = (event.bar_index as number) ?? 0
      const totalBars = (event.total_bars as number) ?? 1
      liveProgress.value = {
        run_id: String(event.run_id ?? ""),
        state: String(event.state ?? ""),
        bar_index: barIndex,
        total_bars: totalBars,
        equity: Number(event.equity ?? 0),
      }
      progressPct.value =
        totalBars > 0
          ? Math.min(100, Math.round((barIndex / totalBars) * 100))
          : 0
      if (event.state === "completed" || event.state === "stopped") {
        setTimeout(loadRuns, 1000)
      }
    },
  )

  onMounted(async () => {
    await loadRuns()
  })

  return {
    cfg,
    liveProgress,
    loadingRuns,
    loadRuns,
    progressPct,
    running,
    runs,
    startRun,
    stopRun,
  }
}
