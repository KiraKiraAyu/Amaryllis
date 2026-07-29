import { onMounted, ref, watch } from "vue"
import {
  getBacktestRunsApi,
  startBacktestApi,
  stopBacktestApi,
} from "@/api/backtest"
import { getModelConfigsApi } from "@/api/models"
import { useRealtimeStore } from "@/stores/realtime"
import type {
  BacktestConfig,
  BacktestLiveProgress,
  BacktestModelOption,
  BacktestRun,
} from "@/types/backtest-ui"

export function useBacktestPage() {
  const realtime = useRealtimeStore()
  const runs = ref<BacktestRun[]>([])
  const loadingRuns = ref(false)
  const running = ref(false)
  const liveProgress = ref<BacktestLiveProgress | null>(null)
  const progressPct = ref(0)
  const modelOptions = ref<BacktestModelOption[]>([])
  const cfg = ref<BacktestConfig>({
    symbols: "BTCUSDT,ETHUSDT",
    interval: "5m",
    startDate: new Date(Date.now() - 7 * 24 * 3600 * 1000)
      .toISOString()
      .slice(0, 10),
    endDate: new Date().toISOString().slice(0, 10),
    initial_balance: 1000,
    fee_bps: 4,
    slippage_bps: 2,
    ai_model_id: "",
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

  async function loadModels() {
    try {
      const data = await getModelConfigsApi()
      const providers = Array.isArray(data?.providers) ? data.providers : []
      modelOptions.value = providers
        .filter((provider) => provider.enabled ?? true)
        .flatMap(
          (provider: {
            name?: string
            models?: { id?: string; name?: string }[]
          }) =>
            (provider.models ?? []).map((model) => ({
              id: model.id ?? "",
              label: `${provider.name ?? "Provider"} / ${model.name ?? model.id ?? ""}`,
            })),
        )

      if (
        !cfg.value.ai_model_id ||
        !modelOptions.value.some((model) => model.id === cfg.value.ai_model_id)
      ) {
        cfg.value.ai_model_id = modelOptions.value[0]?.id ?? ""
      }
    } catch {
      modelOptions.value = []
    }
  }

  async function startRun() {
    running.value = true
    try {
      if (!cfg.value.ai_model_id) {
        running.value = false
        return
      }
      const symbols = cfg.value.symbols
        .split(",")
        .map((symbol) => symbol.trim().toUpperCase())
        .filter(Boolean)
      const startTs = Math.floor(new Date(cfg.value.startDate).getTime() / 1000)
      const endTs = Math.floor(new Date(cfg.value.endDate).getTime() / 1000)
      await startBacktestApi({
        symbols,
        interval: cfg.value.interval,
        start_ts: startTs,
        end_ts: endTs,
        initial_balance: cfg.value.initial_balance,
        fee_bps: cfg.value.fee_bps,
        slippage_bps: cfg.value.slippage_bps,
        ai_model_id: cfg.value.ai_model_id,
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
      progressPct.value = Math.min(
        100,
        Math.round((barIndex / totalBars) * 100),
      )
      if (event.state === "completed" || event.state === "stopped") {
        setTimeout(loadRuns, 1000)
      }
    },
  )

  onMounted(async () => {
    await Promise.all([loadRuns(), loadModels()])
  })

  return {
    cfg,
    liveProgress,
    loadingRuns,
    loadRuns,
    modelOptions,
    progressPct,
    running,
    runs,
    startRun,
    stopRun,
  }
}
