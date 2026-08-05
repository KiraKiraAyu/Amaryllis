import { computed, onMounted, ref, watch } from "vue"
import {
  closeTraderPositionApi,
  getEquityHistoryApi,
  getPositionsApi,
  getTraderAccountApi,
  getTraderListApi,
  startTraderApi,
  stopTraderApi,
  syncTraderBalanceApi,
} from "@/api/trading"
import { useRealtimeStore } from "@/stores/realtime"
import { useToast } from "@/stores/toast"
import { formatTime } from "@/utils/format"
import type {
  DashboardEquitySnapshot,
  DashboardLiveEvent,
  DashboardPosition,
  DashboardTrader,
  EquityChartPoint,
} from "@/types/dashboard-ui"
import type { EquityHistoryPointPayload } from "@/types/public"

export function useDashboardPage() {
  const realtime = useRealtimeStore()
  const toast = useToast()
  const loading = ref(true)
  const initialLoadDone = ref(false)

  const traders = ref<DashboardTrader[]>([])
  const positions = ref<DashboardPosition[]>([])
  const equity = ref<DashboardEquitySnapshot>({
    equity: 0,
    available_cash: 0,
    unrealized_pnl: 0,
    loaded: false,
  })
  const events = ref<DashboardLiveEvent[]>([])
  const equityHistory = ref<EquityChartPoint[]>([])
  const activeChart = ref("")
  const traderIdOptions = computed(() =>
    traders.value.map((trader) => trader.id).slice(0, 5),
  )

  function traderName(id: string) {
    return (
      traders.value.find((trader) => trader.id === id)?.name ||
      `${id.slice(0, 8)}...`
    )
  }

  async function runActionAndReload(action: () => Promise<unknown>) {
    try {
      await action()
    } catch {
      // Request errors are reported by the shared response interceptor.
    } finally {
      await loadAll()
    }
  }

  async function loadEquityHistory(traderId: string) {
    try {
      const points = await getEquityHistoryApi({ trader_id: traderId })
      equityHistory.value = points.map(equityPoint)
    } catch {
      equityHistory.value = []
    }
  }

  async function loadEquity() {
    try {
      const accounts = await Promise.all(
        traders.value.map((t) =>
          getTraderAccountApi({ trader_id: t.id }).catch(() => null),
        ),
      )
      const valid = accounts.filter((a): a is NonNullable<typeof a> => !!a)
      equity.value = {
        equity: valid.reduce((sum, a) => sum + (a.total_balance ?? 0), 0),
        available_cash: valid.reduce(
          (sum, a) => sum + (a.available_balance ?? 0),
          0,
        ),
        unrealized_pnl: valid.reduce(
          (sum, a) => sum + (a.unrealized_pnl ?? 0),
          0,
        ),
        loaded: true,
      }
    } catch {
      equity.value = {
        equity: 0,
        available_cash: 0,
        unrealized_pnl: 0,
        loaded: true,
      }
    }
  }

  async function loadAll() {
    loading.value = true
    try {
      const data = await getTraderListApi()
      traders.value = data.traders

      if (traders.value.length === 0) {
        equity.value = {
          equity: 0,
          available_cash: 0,
          unrealized_pnl: 0,
          loaded: true,
        }
        positions.value = []
        realtime.clearPositions()
        return
      }

      await loadOpenPositions(traders.value.map((trader) => trader.id))
      await loadEquity()

      if (!activeChart.value) {
        const traderId = traders.value[0]!.id
        activeChart.value = traderId
        await loadEquityHistory(traderId)
      }
    } catch {
      traders.value = []
      positions.value = []
      equity.value = {
        equity: 0,
        available_cash: 0,
        unrealized_pnl: 0,
        loaded: true,
      }
    } finally {
      loading.value = false
      initialLoadDone.value = true
    }
  }

  async function startTrader(id: string) {
    await runActionAndReload(() => startTraderApi(id))
  }

  async function stopTrader(id: string) {
    await runActionAndReload(() => stopTraderApi(id))
  }

  async function syncBalance(id: string) {
    await runActionAndReload(() => syncTraderBalanceApi(id))
  }

  async function closePosition(traderId: string, symbol: string, side: string) {
    if (!confirm(`Close ${symbol} position?`)) return
    await runActionAndReload(() =>
      closeTraderPositionApi(traderId, { symbol, side }),
    )
  }

  async function selectEquityTrader(traderId: string) {
    activeChart.value = traderId
    await loadEquityHistory(traderId)
  }

  watch(
    () => realtime.positions,
    (value) => {
      positions.value = value
    },
    { deep: true, immediate: true },
  )
  watch(
    () => realtime.equitySnapshot,
    (value) => {
      if (!value) return
      equity.value = {
        equity: (value.equity as number) ?? 0,
        available_cash: (value.available_cash as number) ?? 0,
        unrealized_pnl: (value.unrealized_pnl as number) ?? 0,
        loaded: true,
      }
    },
  )
  watch(
    () => realtime.lastEvent,
    (event) => {
      if (!event || event.type === "ping") return
      events.value.unshift({
        type: event.type,
        summary: event.trader_id ? `trader:${event.trader_id as string}` : "",
        time: formatTime(Date.now()),
      })
      if (events.value.length > 50) events.value.pop()
    },
  )

  onMounted(() => {
    void loadAll()
    window.setTimeout(() => {
      if (!initialLoadDone.value) {
        loading.value = false
        initialLoadDone.value = true
        equity.value.loaded = true
        toast.error("Dashboard init timed out. Please click Refresh.")
      }
    }, 5000)
  })

  function equityPoint(point: EquityHistoryPointPayload): EquityChartPoint {
    return {
      time: Math.floor(new Date(point.timestamp).getTime() / 1000),
      value: point.total_equity,
    }
  }

  async function loadOpenPositions(traderIds: string[]) {
    const entries = await Promise.all(
      traderIds.map(async (traderId) => {
        try {
          const payload = await getPositionsApi({
            trader_id: traderId,
            status: "open",
          })
          return [payload.trader_id, payload.items] as const
        } catch {
          return [traderId, []] as const
        }
      }),
    )

    realtime.replacePositionsByTrader(
      Object.fromEntries(entries) as Record<string, DashboardPosition[]>,
    )
  }

  return {
    activeChart,
    closePosition,
    equity,
    equityHistory,
    events,
    initialLoadDone,
    loadAll,
    loading,
    positions,
    selectEquityTrader,
    startTrader,
    stopTrader,
    syncBalance,
    traderIdOptions,
    traderName,
    traders,
  }
}
