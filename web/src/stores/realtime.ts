import { defineStore } from "pinia"
import { computed, ref } from "vue"
import type { PositionPayload } from "@/types/trading"
import { SseClient } from "@/utils/sse-client"
import { useAuthStore } from "./auth"

export interface RealtimeEvent {
  type: string
  user_id?: string
  trader_id?: string
  [key: string]: unknown
}

export const useRealtimeStore = defineStore("realtime", () => {
  const connected = ref(false)
  const lastEvent = ref<RealtimeEvent | null>(null)
  const equitySnapshot = ref<Record<string, unknown> | null>(null)
  const positionsByTrader = ref<Record<string, PositionPayload[]>>({})
  const engineStatus = ref<Record<string, string>>({})

  let client: SseClient | null = null

  const isConnected = computed(() => connected.value)
  const positions = computed(() => Object.values(positionsByTrader.value).flat())

  function connect() {
    const auth = useAuthStore()
    if (!auth.token || client) return

    client = new SseClient({
      onMessage: (data) => dispatchEventData(data),
      onOpen: () => {
        connected.value = true
      },
      onClose: () => {
        connected.value = false
      },
      onResponse: (status) => {
        if (status === 401) {
          client?.disconnect()
          useAuthStore().logout()
        }
      },
    })
    client.connect(buildEventsUrl(auth.token))
  }

  function disconnect() {
    client?.disconnect()
    client = null
    connected.value = false
  }

  function dispatchEventData(data: string) {
    if (!data) return
    try {
      const event: RealtimeEvent = JSON.parse(data)
      lastEvent.value = event
      handleEvent(event)
    } catch {
      /* ignore parse errors */
    }
  }

  function buildEventsUrl(token: string) {
    const fallback = new URL("/api/events", window.location.origin)
    const configured = import.meta.env.VITE_EVENTS_URL
    const url = configured ? new URL(configured, fallback) : fallback

    url.searchParams.set("token", token)
    return url.toString()
  }

  function handleEvent(ev: RealtimeEvent) {
    switch (ev.type) {
      case "equity_snapshot":
        equitySnapshot.value = ev as Record<string, unknown>
        break
      case "position_update":
        if (typeof ev.trader_id === "string" && Array.isArray(ev.positions)) {
          setPositionsForTrader(
            ev.trader_id,
            ev.positions.filter(isPositionPayload),
          )
        }
        break
      case "engine_status":
        if (ev.trader_id && typeof ev.status === "string") {
          engineStatus.value[ev.trader_id as string] = ev.status as string
        }
        break
    }
  }

  function setPositionsForTrader(traderId: string, items: PositionPayload[]) {
    positionsByTrader.value = {
      ...positionsByTrader.value,
      [traderId]: items,
    }
  }

  function replacePositionsByTrader(next: Record<string, PositionPayload[]>) {
    positionsByTrader.value = next
  }

  function removePosition(traderId: string, symbol: string, side: string) {
    const current = positionsByTrader.value[traderId] ?? []
    setPositionsForTrader(
      traderId,
      current.filter(
        (position) => position.symbol !== symbol || position.side !== side,
      ),
    )
  }

  function clearPositions() {
    replacePositionsByTrader({})
  }

  function isPositionPayload(value: unknown): value is PositionPayload {
    if (!value || typeof value !== "object") return false
    const position = value as Record<string, unknown>

    return (
      typeof position.id === "string" &&
      typeof position.trader_id === "string" &&
      typeof position.symbol === "string" &&
      typeof position.side === "string" &&
      typeof position.quantity === "number" &&
      typeof position.entry_price === "number" &&
      typeof position.mark_price === "number" &&
      typeof position.liquidation_price === "number" &&
      typeof position.leverage === "number" &&
      typeof position.margin_mode === "string" &&
      typeof position.unrealized_pnl === "number" &&
      typeof position.realized_pnl === "number" &&
      typeof position.status === "string" &&
      typeof position.opened_at === "number" &&
      (position.closed_at === null || typeof position.closed_at === "number") &&
      typeof position.updated_at === "number"
    )
  }

  return {
    connected,
    isConnected,
    lastEvent,
    equitySnapshot,
    positions,
    engineStatus,
    connect,
    clearPositions,
    disconnect,
    removePosition,
    replacePositionsByTrader,
    setPositionsForTrader,
  }
})
