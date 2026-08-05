import { ref, computed, watch, onUnmounted, type Ref } from "vue"
import {
  getTraderApi,
  getDecisionsApi,
  getPositionsApi,
  getRuntimeEventsApi,
  getTraderAccountApi,
  getTraderStatusApi,
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

  /** Unix timestamp (seconds) of the next scheduled scan, or null when
   *  the trader is stopped or a scan is in progress. */
  const nextScanAt = ref<number | null>(null)

  // Status polling: fetch `next_scan_at` from the backend every 2 seconds
  // while the trader is running. The countdown display itself updates every
  // second client-side; this poll just keeps the target timestamp fresh.
  let statusPollTimer: ReturnType<typeof setInterval> | null = null

  async function pollStatus() {
    if (!trader.value?.is_running) return
    try {
      const status = await getTraderStatusApi({ trader_id: traderId.value })
      nextScanAt.value = status.runtime_engine?.next_scan_at ?? null
    } catch {
      /* silent — polling is best-effort */
    }
  }

  function startStatusPolling() {
    stopStatusPolling()
    pollStatus() // immediate fetch
    statusPollTimer = setInterval(pollStatus, 2000)
  }

  function stopStatusPolling() {
    if (statusPollTimer) {
      clearInterval(statusPollTimer)
      statusPollTimer = null
    }
  }

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

      // Fetch initial next_scan_at from runtime engine status
      if (traderRes.is_running) {
        try {
          const status = await getTraderStatusApi({ trader_id: id })
          nextScanAt.value = status.runtime_engine?.next_scan_at ?? null
        } catch {
          /* best-effort */
        }
        startStatusPolling()
      } else {
        nextScanAt.value = null
      }

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
        case "ai_stream_chunk": {
          const chunk = ev.chunk as string
          const symbol = ev.symbol as string
          const correlationId = ev.correlation_id as string

          // Find an existing streaming trader message for this correlation_id
          const existing = feed.value.find(
            (m) =>
              m.role === "trader" &&
              m.streaming === true &&
              m.data?.correlation_id === correlationId,
          )

          if (existing) {
            // Append chunk to the existing streaming message
            existing.content = existing.content + chunk
          } else {
            // Stop any running typewriter — the LLM stream replaces it
            if (typewriterTimer) {
              clearInterval(typewriterTimer)
              typewriterTimer = null
            }
            // Create a new streaming trader message
            feed.value.push({
              id: `stream-${correlationId}`,
              role: "trader",
              title: `AI Thinking: ${symbol}…`,
              content: chunk,
              timestamp: Date.now(),
              streaming: true,
              data: {
                symbol,
                correlation_id: correlationId,
              },
            })
            typing.value = true
          }
          break
        }
        case "ai_decision": {
          const decision = ev.decision as Record<string, unknown> | undefined
          const correlationId = decision?.correlation_id as string | undefined

          // Try to find and finalize an existing streaming message
          let streamingMsg: FeedMessage | undefined
          if (correlationId) {
            streamingMsg = feed.value.find(
              (m) =>
                m.role === "trader" &&
                m.streaming === true &&
                m.data?.correlation_id === correlationId,
            )
          }

          if (streamingMsg) {
            // Update the streaming message with final structured data
            streamingMsg.title = `AI Decision: ${decision?.symbol ?? "?"} → ${decision?.action ?? "?"}`
            streamingMsg.content =
              (decision?.reason as string) || streamingMsg.content
            streamingMsg.streaming = false
            streamingMsg.data = {
              symbol: decision?.symbol,
              decision: decision?.action,
              confidence: decision?.confidence,
              timeframe: decision?.timeframe,
              correlation_id: correlationId,
            }
            typing.value = false
          } else {
            // No streaming message found — create a new one (fallback)
            addMessage({
              id: `ai-${Date.now()}`,
              role: "trader",
              title: `AI Decision: ${decision?.symbol ?? "?"} → ${decision?.action ?? "?"}`,
              content:
                (decision?.reasoning as string) ||
                (decision?.reason as string) ||
                "",
              timestamp: Date.now(),
              data: {
                symbol: decision?.symbol,
                decision: decision?.action,
                confidence: decision?.confidence,
                timeframe: decision?.timeframe,
              },
            })
          }
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
          // Start/stop polling based on engine status changes
          if (status === "running") {
            startStatusPolling()
          } else if (status === "stopped" || status === "budget_exhausted") {
            stopStatusPolling()
            nextScanAt.value = null
          }
          break
        }
        case "scan_schedule": {
          // Real-time update from backend: next_scan_at changed
          nextScanAt.value = (ev.next_scan_at as number | null | undefined) ?? null
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
      startStatusPolling()
    } catch {
      /* handled by interceptor */
    }
  }

  async function stopTrader() {
    try {
      await stopTraderApi(traderId.value)
      toast.success("Trader stopped")
      if (trader.value) trader.value.is_running = false
      stopStatusPolling()
      nextScanAt.value = null
    } catch {
      /* handled by interceptor */
    }
  }

  onUnmounted(() => {
    if (typewriterTimer) clearInterval(typewriterTimer)
    stopStatusPolling()
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
    nextScanAt,
    loadAll,
    startTrader,
    stopTrader,
  }
}
