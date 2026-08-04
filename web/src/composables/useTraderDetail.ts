import { ref, computed, watch, onUnmounted, type Ref } from "vue"
import {
  getTraderApi,
  getDecisionsApi,
  getPositionsApi,
  getRuntimeEventsApi,
  getTraderAccountApi,
  startTraderApi,
  stopTraderApi,
} from "@/api/trading"
import { useRealtimeStore } from "@/stores/realtime"
import { useToast } from "@/stores/toast"
import type {
  TraderPayload,
  PositionPayload,
  TraderAccountPayload,
  RuntimeEventPayload,
} from "@/types/trading"

/** A single chat-like message in the trader activity feed. */
export interface FeedMessage {
  id: string
  /** "prompt" = system prompt to AI (right side);
   *  "trader" = AI reasoning (left side);
   *  "action" = trade execution (left side);
   *  "position" = position update (left side);
   *  "system" = system operation (left side, hidden by default). */
  role: "prompt" | "system" | "trader" | "action" | "position"
  title: string
  content: string
  timestamp: number
  /** Whether the typewriter animation is still streaming. */
  streaming?: boolean
  /** Additional structured data to render (e.g. positions, decisions). */
  data?: Record<string, unknown>
}

export function useTraderDetail(traderId: Ref<string>) {
  const toast = useToast()
  const realtime = useRealtimeStore()

  const trader = ref<TraderPayload | null>(null)
  const account = ref<TraderAccountPayload | null>(null)
  const positions = ref<PositionPayload[]>([])
  const loading = ref(true)
  const error = ref("")
  const feed = ref<FeedMessage[]>([])
  const typing = ref(false)
  const showSystemOps = ref(false)

  /** Feed filtered by the system-ops toggle. System Ops (action / system)
   *  messages are hidden by default; the user can toggle them on in the UI. */
  const filteredFeed = computed(() =>
    showSystemOps.value
      ? feed.value
      : feed.value.filter((m) => m.role !== "system" && m.role !== "action"),
  )

  // Typewriter state
  let typewriterTimer: ReturnType<typeof setInterval> | null = null

  /** Load trader info, account, positions, historical decisions and events. */
  async function loadAll() {
    loading.value = true
    error.value = ""
    try {
      const id = traderId.value

      const [traderRes, positionsRes, decisionsRes, eventsRes, accountRes] =
        await Promise.all([
          getTraderApi(id),
          getPositionsApi({ trader_id: id, status: "open" }),
          getDecisionsApi({ trader_id: id, limit: 20 }),
          getRuntimeEventsApi({ trader_id: id, limit: 50 }),
          getTraderAccountApi({ trader_id: id }).catch(() => null),
        ])

      trader.value = traderRes
      positions.value = positionsRes.items ?? []
      account.value = accountRes

      // Build chronological feed from decisions + events
      const messages: FeedMessage[] = []

      // Add decisions as "trader" messages, with their prompts as "prompt" messages
      for (const d of (decisionsRes.items ?? []).reverse()) {
        // Parse payload_json to extract the prompt
        let payload: Record<string, unknown> = {}
        try {
          payload = JSON.parse(d.payload_json) as Record<string, unknown>
        } catch {
          /* ignore parse errors */
        }

        const promptText = payload.prompt as string | undefined
        const systemPrompt = payload.system_prompt as string | undefined

        // If a prompt exists, add it as a right-side "prompt" message
        if (promptText && promptText.trim()) {
          const content = systemPrompt
            ? `[System Prompt]\n${systemPrompt}\n\n[User Message]\n${promptText}`
            : promptText
          messages.push({
            id: `prompt-${d.id}`,
            role: "prompt",
            title: `Prompt → ${d.symbol}`,
            content,
            timestamp: d.created_at - 1, // 1ms before the decision so it appears above
          })
        }

        messages.push({
          id: `decision-${d.id}`,
          role: "trader",
          title: `AI Decision: ${d.symbol} → ${d.decision}`,
          content: d.reason || "",
          timestamp: d.created_at,
          data: {
            symbol: d.symbol,
            decision: d.decision,
            confidence: d.confidence,
            timeframe: d.timeframe,
          },
        })
      }

      // Add runtime events as "system" or "action" messages
      for (const e of (eventsRes.items ?? []).reverse()) {
        const isAction = e.action_taken && e.action_taken !== ""
        messages.push({
          id: `event-${e.id}`,
          role: isAction ? "action" : "system",
          title: formatEventTitle(e),
          content: e.action_taken || e.event_type,
          timestamp: e.created_at,
          data: {
            event_type: e.event_type,
            symbol: e.symbol,
            side: e.side,
            risk_level: e.risk_level,
            payload: e.payload,
          },
        })
      }

      // Sort by timestamp ascending
      messages.sort((a, b) => a.timestamp - b.timestamp)
      feed.value = messages
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : "Failed to load trader"
    } finally {
      loading.value = false
    }
  }

  function formatEventTitle(e: RuntimeEventPayload): string {
    const parts = [e.event_type]
    if (e.symbol) parts.push(e.symbol)
    if (e.side) parts.push(e.side)
    return parts.join(" · ")
  }

  /** Start typewriter animation for a new message. */
  function startTypewriter(msg: FeedMessage) {
    if (typewriterTimer) clearInterval(typewriterTimer)

    msg.streaming = true
    typing.value = true
    const fullText = msg.content
    let charIndex = 0

    typewriterTimer = setInterval(() => {
      if (charIndex >= fullText.length) {
        if (typewriterTimer) clearInterval(typewriterTimer)
        typewriterTimer = null
        msg.streaming = false
        typing.value = false
        return
      }
      charIndex++
      msg.content = fullText.slice(0, charIndex)
    }, 20)
  }

  /** Add a new message to the feed (with typewriter if it's from trader). */
  function addMessage(msg: FeedMessage) {
    feed.value.push(msg)
    if (msg.role === "trader" && msg.content) {
      // Use the proxied version from the reactive array so mutations trigger reactivity
      const proxiedMsg = feed.value[feed.value.length - 1]
      startTypewriter(proxiedMsg)
    }
  }

  // Watch realtime events for this trader
  const stopWatch = watch(
    () => realtime.lastEvent,
    (ev) => {
      if (!ev || ev.trader_id !== traderId.value) return

      switch (ev.type) {
        case "ai_prompt": {
          const prompt = ev.prompt as string | undefined
          const systemPrompt = ev.system_prompt as string | undefined
          const symbol = ev.symbol as string | undefined
          const content = systemPrompt
            ? `[System Prompt]\n${systemPrompt}\n\n[User Message]\n${prompt || ""}`
            : prompt || ""
          addMessage({
            id: `prompt-${Date.now()}`,
            role: "prompt",
            title: symbol ? `Prompt → ${symbol}` : "System Prompt",
            content,
            timestamp: Date.now(),
          })
          break
        }
        case "ai_decision": {
          const decision = ev.decision as Record<string, unknown> | undefined
          addMessage({
            id: `ai-${Date.now()}`,
            role: "trader",
            title: `AI Decision: ${decision?.symbol ?? "?"} → ${decision?.action ?? "?"}`,
            content:
              (decision?.reasoning as string) ||
              (decision?.reason as string) ||
              "",
            timestamp: Date.now(),
            data: decision,
          })
          break
        }
        case "trade_execution": {
          const trade = ev.trade as Record<string, unknown> | undefined
          addMessage({
            id: `trade-${Date.now()}`,
            role: "action",
            title: `Trade: ${trade?.side ?? "?"} ${trade?.symbol ?? "?"}`,
            content: `Order ${trade?.side ?? ""} ${trade?.quantity ?? ""} ${trade?.symbol ?? ""} @ ${trade?.price ?? ""}`,
            timestamp: Date.now(),
            data: trade,
          })
          break
        }
        case "position_update": {
          const positions = ev.positions as PositionPayload[] | undefined
          if (positions && positions.length > 0) {
            addMessage({
              id: `pos-${Date.now()}`,
              role: "position",
              title: `Position Update: ${positions.length} position(s)`,
              content: positions
                .map(
                  (p) =>
                    `${p.symbol} ${p.side} qty=${p.quantity} uPnL=${p.unrealized_pnl.toFixed(2)}`,
                )
                .join("; "),
              timestamp: Date.now(),
              data: { positions },
            })
          }
          // Also update local positions array
          if (Array.isArray(positions)) {
            updatePositions(positions)
          }
          break
        }
        case "engine_status": {
          const status = ev.status as string
          const message = ev.message as string
          addMessage({
            id: `engine-${Date.now()}`,
            role: "system",
            title: `Engine: ${status}`,
            content: message || `Engine status: ${status}`,
            timestamp: Date.now(),
          })
          break
        }
      }
    },
  )

  function updatePositions(newPositions: PositionPayload[]) {
    // Merge: update existing, add new
    const map = new Map(positions.value.map((p) => [p.id, p]))
    for (const p of newPositions) {
      map.set(p.id, p)
    }
    positions.value = Array.from(map.values())
  }

  async function startTrader() {
    try {
      await startTraderApi(traderId.value)
      toast.success("Trader started")
      if (trader.value) trader.value.is_running = true
    } catch {
      /* handled by interceptor */
    }
  }

  async function stopTrader() {
    try {
      await stopTraderApi(traderId.value)
      toast.success("Trader stopped")
      if (trader.value) trader.value.is_running = false
    } catch {
      /* handled by interceptor */
    }
  }

  onUnmounted(() => {
    if (typewriterTimer) clearInterval(typewriterTimer)
    stopWatch()
  })

  return {
    trader,
    account,
    positions,
    loading,
    error,
    feed,
    filteredFeed,
    typing,
    showSystemOps,
    loadAll,
    startTrader,
    stopTrader,
  }
}
