import { computed, onMounted, ref } from "vue"
import { getCompetitionApi, getEquityHistoryApi } from "@/api/competition"
import {
  deleteTraderApi,
  getTraderListApi,
  startTraderApi,
  stopTraderApi,
  syncTraderBalanceApi,
} from "@/api/trading"
import { useToast } from "@/stores/toast"
import type { TraderPayload } from "@/types/trading"
import type { EquityHistoryPointPayload } from "@/types/public"

export interface TradersPageTrader extends TraderPayload {
  total_equity: number
  total_pnl: number
  total_pnl_pct: number
  position_count: number
  margin_used_pct: number
}

export interface EquityPoint {
  time: number
  value: number
}

export function useTradersPage() {
  const toast = useToast()
  const loading = ref(true)
  const traders = ref<TradersPageTrader[]>([])
  const search = ref("")
  const lastUpdated = ref("-")
  const selectedTrader = ref<TradersPageTrader | null>(null)
  const selectedEquity = ref<EquityPoint[]>([])

  const avatarColors = ["#e2528a", "#9b6dae", "#5b8dee", "#2ecc71", "#e67e22"]

  const topThree = computed(() =>
    [...traders.value]
      .sort((a, b) => b.total_pnl_pct - a.total_pnl_pct)
      .slice(0, 3),
  )

  const filtered = computed(() => {
    if (!search.value) return traders.value
    const query = search.value.toLowerCase()
    return traders.value.filter(
      (trader) =>
        trader.name.toLowerCase().includes(query) ||
        trader.ai_model_id.toLowerCase().includes(query),
    )
  })

  const stats = computed(() => ({
    total: traders.value.length,
    running: traders.value.filter((t) => t.is_running).length,
    stopped: traders.value.filter((t) => !t.is_running).length,
    totalEquity: traders.value.reduce(
      (sum, t) => sum + (t.total_equity || 0),
      0,
    ),
  }))

  function fmt(value: number) {
    return (value ?? 0).toLocaleString("en-US", {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    })
  }

  function returnPct(trader: TradersPageTrader) {
    return trader.total_pnl_pct
  }

  function avatarStyle(id: string) {
    const color = avatarColors[id.charCodeAt(0) % avatarColors.length]
    return `background-color:${color}22;color:${color};`
  }

  async function load() {
    loading.value = true
    try {
      const [traderList, competitionData] = await Promise.all([
        getTraderListApi(),
        getCompetitionApi().catch(() => ({ traders: [], count: 0 })),
      ])

      const competitionMap = new Map(
        competitionData.traders.map((t) => [t.trader_id, t]),
      )

      traders.value = traderList.traders.map((trader) => {
        const comp = competitionMap.get(trader.id)
        return {
          ...trader,
          total_equity: comp?.total_equity ?? 0,
          total_pnl: comp?.total_pnl ?? 0,
          total_pnl_pct: comp?.total_pnl_pct ?? 0,
          position_count: comp?.position_count ?? 0,
          margin_used_pct: comp?.margin_used_pct ?? 0,
        }
      })

      lastUpdated.value = new Date().toLocaleTimeString()
    } catch {
      toast.error("Failed to load traders")
    } finally {
      loading.value = false
    }
  }

  async function startTrader(id: string) {
    try {
      await startTraderApi(id)
      toast.success("Trader started")
      await load()
    } catch {
      /* error handled by interceptor */
    }
  }

  async function stopTrader(id: string) {
    try {
      await stopTraderApi(id)
      toast.success("Trader stopped")
      await load()
    } catch {
      /* error handled by interceptor */
    }
  }

  async function syncBalance(id: string) {
    try {
      await syncTraderBalanceApi(id)
      toast.success("Balance synced")
      await load()
    } catch {
      /* error handled by interceptor */
    }
  }

  async function deleteTrader(id: string) {
    if (!confirm("Are you sure you want to delete this trader?")) return
    try {
      await deleteTraderApi(id)
      toast.success("Trader deleted")
      await load()
    } catch {
      /* error handled by interceptor */
    }
  }

  async function showDetail(trader: TradersPageTrader) {
    selectedTrader.value = trader
    selectedEquity.value = []
    try {
      const points = await getEquityHistoryApi({ trader_id: trader.id })
      selectedEquity.value = points.map(equityPoint)
    } catch {
      /* keep empty detail chart */
    }
  }

  function equityPoint(point: EquityHistoryPointPayload): EquityPoint {
    return {
      time: Math.floor(new Date(point.timestamp).getTime() / 1000),
      value: point.total_equity,
    }
  }

  onMounted(load)

  return {
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
    topThree,
    traders,
  }
}
