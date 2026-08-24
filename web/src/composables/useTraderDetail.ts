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
  wakeTraderApi,
} from "@/api/trading"
import { useRealtimeStore } from "@/stores/realtime"
import { useToast } from "@/stores/toast"
import { fmtUsd } from "@/utils/format"
import type {
  TraderPayload,
  PositionPayload,
  TraderAccountPayload,
  DecisionPayload,
  RuntimeEventPayload,
} from "@/types/trading"

const LIVE_OPEN_SKIPPED_CONSTRAINTS_EVENT = "live_open_skipped_constraints"
const ACTIVITY_POLL_INTERVAL_MS = 5_000
const ACTIVITY_LIMIT = 100
const ACTIVITY_WINDOW_HOURS = 24 * 365

/** A single entry in the trader activity timeline. */
export interface FeedMessage {
  id: string
  /** "prompt" = system prompt sent to AI;
   *  "trader" = AI reasoning + decision (+ optional execution data);
   *  "system" = system operation (hidden by default, toggled by System Ops);
   *  "warning" = action blocked by an exchange constraint (always visible). */
  role: "prompt" | "system" | "trader" | "warning"
  title: string
  content: string
  timestamp: number
  /** Whether the typewriter animation is still streaming. */
  streaming?: boolean
  /** Additional structured data to render (e.g. decision, execution). */
  data?: Record<string, unknown>
}

function payloadNumber(
  payload: Record<string, unknown>,
  key: string,
): number | null {
  const value = payload[key]
  return typeof value === "number" && Number.isFinite(value) ? value : null
}

function formatQuantity(value: number | null): string {
  if (value == null) return "unknown"
  return value.toLocaleString("en-US", { maximumFractionDigits: 8 })
}

function formatConstraintSkipContent(event: RuntimeEventPayload): string {
  const payload = event.payload
  const symbol = event.symbol || "Order"
  const side = event.side || ""
  const action = side ? `${symbol} ${side}` : symbol
  const configuredCost = payloadNumber(payload, "configured_cost")
  const configuredCostText =
    configuredCost == null
      ? "Calculated"
      : `Configured cost $${fmtUsd(configuredCost)} yields calculated`
  const minimumCost = payloadNumber(payload, "minimum_required_cost")
  const minimumCostText =
    minimumCost == null
      ? ""
      : ` Required minimum cost is $${fmtUsd(minimumCost)}.`

  if (payload.reason === "quantity_below_minimum") {
    return `${action} was not submitted. ${configuredCostText} quantity ${formatQuantity(
      payloadNumber(payload, "raw_quantity"),
    )} is below the exchange minimum ${formatQuantity(
      payloadNumber(payload, "minimum_quantity"),
    )}.${minimumCostText}`
  }

  if (payload.reason === "notional_below_minimum") {
    const notional = payloadNumber(payload, "normalized_notional")
    const minimumNotional = payloadNumber(payload, "minimum_notional")
    return `${action} was not submitted. ${configuredCostText} notional $${fmtUsd(
      notional ?? 0,
    )} is below the exchange minimum $${fmtUsd(minimumNotional ?? 0)}.${minimumCostText}`
  }

  return `${action} was not submitted because the calculated order did not meet the exchange minimum requirements.${minimumCostText}`
}

function formatEventTitle(event: RuntimeEventPayload): string {
  const parts = [event.event_type]
  if (event.symbol) parts.push(event.symbol)
  if (event.side) parts.push(event.side)
  return parts.join(" - ")
}

function runtimeEventToFeedMessage(event: RuntimeEventPayload): FeedMessage {
  const isConstraintWarning =
    event.event_type === LIVE_OPEN_SKIPPED_CONSTRAINTS_EVENT

  return {
    id: `event-${event.id}`,
    role: isConstraintWarning ? "warning" : "system",
    title: isConstraintWarning
      ? `Order not submitted: ${event.symbol} ${event.side}`.trim()
      : formatEventTitle(event),
    content: isConstraintWarning
      ? formatConstraintSkipContent(event)
      : event.action_taken || event.event_type,
    timestamp: event.created_at,
    data: {
      event_type: event.event_type,
      symbol: event.symbol,
      side: event.side,
      risk_level: event.risk_level,
      payload: event.payload,
    },
  }
}

function isRuntimeEventPayload(value: unknown): value is RuntimeEventPayload {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false
  const event = value as Record<string, unknown>

  return (
    typeof event.id === "string" &&
    typeof event.event_type === "string" &&
    typeof event.symbol === "string" &&
    typeof event.side === "string" &&
    typeof event.risk_level === "string" &&
    typeof event.trigger_source === "string" &&
    typeof event.action_taken === "string" &&
    typeof event.correlation_id === "string" &&
    typeof event.created_at === "number" &&
    !!event.payload &&
    typeof event.payload === "object" &&
    !Array.isArray(event.payload)
  )
}

function decisionToFeedMessages(decision: DecisionPayload): FeedMessage[] {
  let payload: Record<string, unknown> = {}
  try {
    const parsed: unknown = JSON.parse(decision.payload_json)
    if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
      payload = parsed as Record<string, unknown>
    }
  } catch {
    /* ignore malformed historical payloads */
  }

  const messages: FeedMessage[] = []
  const promptText = payload.prompt as string | undefined
  const systemPrompt = payload.system_prompt as string | undefined
  const decisionStartedAt =
    payloadNumber(payload, "decision_started_at") ?? decision.created_at
  const decisionCompletedAt =
    payloadNumber(payload, "completed_at") ?? decision.created_at

  if (promptText && promptText.trim()) {
    messages.push({
      id: `prompt-${decision.id}`,
      role: "prompt",
      title: `Prompt -> ${decision.symbol}`,
      content: systemPrompt
        ? `[System Prompt]\n${systemPrompt}\n\n[User Message]\n${promptText}`
        : promptText,
      timestamp: decisionStartedAt,
    })
  }

  messages.push({
    id: `decision-${decision.id}`,
    role: "trader",
    title: `AI Decision: ${decision.symbol} -> ${decision.decision}`,
    content: decision.reason || "",
    timestamp: decisionCompletedAt,
    data: {
      symbol: decision.symbol,
      decision: decision.decision,
      confidence: decision.confidence,
      timeframe: decision.timeframe,
      correlation_id: payload.correlation_id,
    },
  })

  return messages
}

function feedRoleOrder(role: FeedMessage["role"]): number {
  switch (role) {
    case "prompt":
      return 0
    case "trader":
      return 1
    case "warning":
      return 2
    case "system":
      return 3
    default:
      return 4
  }
}

export function buildPersistedFeed(
  decisions: DecisionPayload[],
  events: RuntimeEventPayload[],
): FeedMessage[] {
  const messages = [
    ...decisions.flatMap(decisionToFeedMessages),
    ...events.map(runtimeEventToFeedMessage),
  ]

  return messages.sort(
    (left, right) =>
      left.timestamp - right.timestamp ||
      feedRoleOrder(left.role) - feedRoleOrder(right.role) ||
      left.id.localeCompare(right.id),
  )
}

function sameFeed(left: FeedMessage[], right: FeedMessage[]): boolean {
  return (
    left.length === right.length &&
    left.every((message, index) => {
      const other = right[index]
      return (
        other !== undefined &&
        message.id === other.id &&
        message.role === other.role &&
        message.title === other.title &&
        message.content === other.content &&
        message.timestamp === other.timestamp
      )
    })
  )
}

function nowSeconds(): number {
  return Math.floor(Date.now() / 1_000)
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
  const waking = ref(false)

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

  /** Feed filtered by the system-ops toggle. System Ops messages are
   *  hidden by default; the user can toggle them on in the UI. */
  const filteredFeed = computed(() =>
    showSystemOps.value
      ? feed.value
      : feed.value.filter((m) => m.role !== "system"),
  )

  // Typewriter state
  let typewriterTimer: ReturnType<typeof setInterval> | null = null
  let activityPollTimer: ReturnType<typeof setInterval> | null = null
  let activityRefreshTimer: ReturnType<typeof setTimeout> | null = null
  let activityRequestId = 0

  async function refreshActivityFeed() {
    const requestId = ++activityRequestId
    const id = traderId.value
    const [decisionsRes, eventsRes] = await Promise.all([
      getDecisionsApi({ trader_id: id, limit: ACTIVITY_LIMIT }),
      getRuntimeEventsApi({
        trader_id: id,
        limit: ACTIVITY_LIMIT,
        window_hours: ACTIVITY_WINDOW_HOURS,
      }),
    ])

    if (requestId !== activityRequestId || id !== traderId.value) return

    const nextFeed = buildPersistedFeed(
      decisionsRes.items ?? [],
      eventsRes.items ?? [],
    )
    if (sameFeed(feed.value, nextFeed)) return

    if (typewriterTimer) {
      clearInterval(typewriterTimer)
      typewriterTimer = null
    }
    typing.value = false
    feed.value = nextFeed
  }

  function scheduleActivityRefresh() {
    if (activityRefreshTimer) return

    activityRefreshTimer = setTimeout(() => {
      activityRefreshTimer = null
      void refreshActivityFeed().catch(() => undefined)
    }, 250)
  }

  function startActivityPolling() {
    stopActivityPolling()
    activityPollTimer = setInterval(() => {
      void refreshActivityFeed().catch(() => undefined)
    }, ACTIVITY_POLL_INTERVAL_MS)
  }

  function stopActivityPolling() {
    if (activityPollTimer) {
      clearInterval(activityPollTimer)
      activityPollTimer = null
    }
    if (activityRefreshTimer) {
      clearTimeout(activityRefreshTimer)
      activityRefreshTimer = null
    }
  }

  /** Load trader info, account, positions, historical decisions and events. */
  async function loadAll() {
    stopActivityPolling()
    activityRequestId++
    loading.value = true
    error.value = ""
    try {
      const id = traderId.value

      const [traderRes, positionsRes, accountRes] = await Promise.all([
        getTraderApi(id),
        getPositionsApi({ trader_id: id, status: "open" }),
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

      await refreshActivityFeed()
      startActivityPolling()
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : "Failed to load trader"
    } finally {
      loading.value = false
    }
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
            timestamp: nowSeconds(),
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
              timestamp: nowSeconds(),
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
          const completedAt =
            decision && typeof decision.completed_at === "number"
              ? decision.completed_at
              : nowSeconds()

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
            streamingMsg.timestamp = completedAt
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
              timestamp: completedAt,
              data: {
                symbol: decision?.symbol,
                decision: decision?.action,
                confidence: decision?.confidence,
                timeframe: decision?.timeframe,
              },
            })
          }
          scheduleActivityRefresh()
          break
        }
        case "trade_execution": {
          const trade = ev.trade as Record<string, unknown> | undefined
          // Attach execution data to the most recent trader message
          const lastTrader = [...feed.value]
            .reverse()
            .find((m) => m.role === "trader")
          if (lastTrader) {
            lastTrader.data = { ...lastTrader.data, execution: trade }
          }
          scheduleActivityRefresh()
          break
        }
        case "runtime_event": {
          if (isRuntimeEventPayload(ev.event)) {
            const message = runtimeEventToFeedMessage(ev.event)
            if (!feed.value.some((existing) => existing.id === message.id)) {
              addMessage(message)
            }
          }
          scheduleActivityRefresh()
          break
        }
        case "position_update": {
          const positions = ev.positions as PositionPayload[] | undefined
          // Positions are real-time data shown in the stats bar, not in the timeline
          if (Array.isArray(positions)) {
            updatePositions(positions)
          }
          break
        }
        case "engine_status": {
          const status = typeof ev.status === "string" ? ev.status : "unknown"
          const message = typeof ev.message === "string" ? ev.message : ""
          // Engine errors (cycle_error, budget_exhausted, etc.) should always be visible
          const isEngineError =
            status === "cycle_error" ||
            status === "budget_exhausted" ||
            status.startsWith("error")
          addMessage({
            id: `engine-${Date.now()}`,
            role: isEngineError ? "warning" : "system",
            title: `Engine: ${status}`,
            content: message || `Engine status: ${status}`,
            timestamp: nowSeconds(),
          })
          // Start/stop polling based on engine status changes
          if (status === "running") {
            startStatusPolling()
          } else if (status === "stopped" || status === "budget_exhausted") {
            stopStatusPolling()
            nextScanAt.value = null
            if (trader.value) trader.value.is_running = false
          }
          break
        }
        case "scan_schedule": {
          // Real-time update from backend: next_scan_at changed
          nextScanAt.value =
            (ev.next_scan_at as number | null | undefined) ?? null
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

  async function wakeTrader() {
    if (waking.value) return

    waking.value = true
    try {
      await wakeTraderApi(traderId.value)
      toast.success("Wake requested")
    } catch {
      /* handled by interceptor */
    } finally {
      waking.value = false
    }
  }

  onUnmounted(() => {
    if (typewriterTimer) clearInterval(typewriterTimer)
    stopActivityPolling()
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
    wakeTrader,
    waking,
  }
}
