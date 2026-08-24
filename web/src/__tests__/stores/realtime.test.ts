import { describe, it, expect, beforeEach, vi } from "vitest"
import { setActivePinia, createPinia } from "pinia"
import { useRealtimeStore } from "@/stores/realtime"
import type { PositionPayload } from "@/types/trading"

vi.mock("@/router", () => ({
  default: {
    push: vi.fn<() => Promise<void>>(),
  },
}))

describe("Realtime Store", () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  const samplePosition: PositionPayload = {
    id: "pos_1",
    trader_id: "trader_1",
    symbol: "BTCUSDT",
    side: "LONG",
    quantity: 0.5,
    entry_price: 60000,
    mark_price: 61000,
    liquidation_price: 45000,
    leverage: 5,
    margin_mode: "cross",
    unrealized_pnl: 500,
    realized_pnl: 0,
    tp_price: null,
    sl_price: null,
    status: "open",
    opened_at: 1700000000,
    closed_at: null,
    updated_at: 1700000000,
  }

  it("initializes with disconnected state", () => {
    const store = useRealtimeStore()
    expect(store.connected).toBe(false)
    expect(store.connecting).toBe(false)
    expect(store.isConnected).toBe(false)
    expect(store.positions).toEqual([])
  })

  it("stores and queries positions by trader", () => {
    const store = useRealtimeStore()

    store.setPositionsForTrader("trader_1", [samplePosition])

    expect(store.positions.length).toBe(1)
    expect(store.positions[0]?.symbol).toBe("BTCUSDT")
  })

  it("removes a position by trader, symbol and side", () => {
    const store = useRealtimeStore()
    store.setPositionsForTrader("trader_1", [samplePosition])

    store.removePosition("trader_1", "BTCUSDT", "LONG")

    expect(store.positions.length).toBe(0)
  })

  it("replaces and clears all positions across traders", () => {
    const store = useRealtimeStore()
    store.replacePositionsByTrader({
      trader_1: [samplePosition],
      trader_2: [{ ...samplePosition, id: "pos_2", trader_id: "trader_2" }],
    })

    expect(store.positions.length).toBe(2)

    store.clearPositions()
    expect(store.positions.length).toBe(0)
  })
})
