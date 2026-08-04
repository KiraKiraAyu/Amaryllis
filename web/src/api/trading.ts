import type {
  ClosePositionPayload,
  ClosePositionRequest,
  CreateTraderRequest,
  DecisionListPayload,
  DecisionQuery,
  FillListPayload,
  GridRiskInfoPayload,
  LatestDecisionsPayload,
  OrderListPayload,
  PaginationQuery,
  PositionListPayload,
  PositionQuery,
  RuntimeAlertAckPayload,
  RuntimeAlertAckRequest,
  RuntimeAlertControlTargetRequest,
  RuntimeAlertControlsPayload,
  RuntimeAlertControlsQuery,
  RuntimeAlertDeliveriesPayload,
  RuntimeAlertDeliveriesQuery,
  RuntimeAlertHistoryPayload,
  RuntimeAlertHistoryQuery,
  RuntimeAlertMutePayload,
  RuntimeAlertMuteRequest,
  RuntimeAlertsPayload,
  RuntimeAlertsQuery,
  RuntimeEventTypesPayload,
  RuntimeEventTypesQuery,
  RuntimeEventsPayload,
  RuntimeEventsQuery,
  RuntimeMetricsPayload,
  RuntimeMetricsQuery,
  RuntimeMetricsSeriesPayload,
  RuntimeMetricsSeriesQuery,
  StatisticsQuery,
  TradeListPayload,
  TraderAccountPayload,
  TraderBalanceSyncPayload,
  TraderCreatedPayload,
  TraderListPayload,
  TraderMessagePayload,
  TraderPayload,
  TraderQuery,
  TraderStatisticsPayload,
  TraderStatusPayload,
  UpdatePromptRequest,
  UpdateTraderRequest,
} from "@/types/trading"
import type { EquityHistoryPointPayload, EquityHistoryQuery } from "@/types/public"
import request from "@/utils/request"

const Api = {
  Traders: "/api/trading/traders",
  TraderDetail: "/api/trading/traders/{id}",
  TraderConfig: "/api/trading/traders/{id}/config",
  TraderStart: "/api/trading/traders/{id}/start",
  TraderStop: "/api/trading/traders/{id}/stop",
  TraderPrompt: "/api/trading/traders/{id}/prompt",
  TraderSyncBalance: "/api/trading/traders/{id}/sync-balance",
  TraderClosePosition: "/api/trading/traders/{id}/close-position",
  TraderGridRisk: "/api/trading/traders/{id}/grid-risk",
  EquityHistory: "/api/trading/equity-history",
  Status: "/api/trading/status",
  Account: "/api/trading/account",
  Positions: "/api/trading/positions",
  PositionsHistory: "/api/trading/positions/history",
  Trades: "/api/trading/trades",
  Orders: "/api/trading/orders",
  OrderFills: "/api/trading/orders/{id}/fills",
  OpenOrders: "/api/trading/open-orders",
  Decisions: "/api/trading/decisions",
  LatestDecisions: "/api/trading/decisions/latest",
  Statistics: "/api/trading/statistics",
  RuntimeMetrics: "/api/trading/runtime-metrics",
  RuntimeMetricsSeries: "/api/trading/runtime-metrics-series",
  RuntimeAlerts: "/api/trading/runtime-alerts",
  RuntimeAlertHistory: "/api/trading/runtime-alert-history",
  RuntimeAlertDeliveries: "/api/trading/runtime-alert-deliveries",
  RuntimeAlertControls: "/api/trading/runtime-alert-controls",
  RuntimeAlertMute: "/api/trading/runtime-alert-controls/mute",
  RuntimeAlertUnmute: "/api/trading/runtime-alert-controls/unmute",
  RuntimeAlertAck: "/api/trading/runtime-alert-controls/ack",
  RuntimeEvents: "/api/trading/runtime-events",
  RuntimeEventTypes: "/api/trading/runtime-event-types",
} as const

export function getTraderListApi() {
  return request.get<TraderListPayload>(Api.Traders)
}

export function createTraderApi(data: CreateTraderRequest) {
  return request.post<TraderCreatedPayload>(Api.Traders, data)
}

export function getTraderApi(id: string) {
  return request.get<TraderPayload>(Api.TraderDetail.replace("{id}", id))
}

export function updateTraderApi(id: string, data: UpdateTraderRequest) {
  return request.put<TraderMessagePayload>(
    Api.TraderDetail.replace("{id}", id),
    data,
  )
}

export function deleteTraderApi(id: string) {
  return request.delete<TraderMessagePayload>(
    Api.TraderDetail.replace("{id}", id),
  )
}

export function getTraderConfigApi(id: string) {
  return request.get<TraderPayload>(Api.TraderConfig.replace("{id}", id))
}

export function startTraderApi(id: string) {
  return request.post<TraderMessagePayload>(Api.TraderStart.replace("{id}", id))
}

export function stopTraderApi(id: string) {
  return request.post<TraderMessagePayload>(Api.TraderStop.replace("{id}", id))
}

export function updateTraderPromptApi(id: string, data: UpdatePromptRequest) {
  return request.put<TraderMessagePayload>(
    Api.TraderPrompt.replace("{id}", id),
    data,
  )
}

export function syncTraderBalanceApi(id: string) {
  return request.post<TraderBalanceSyncPayload>(
    Api.TraderSyncBalance.replace("{id}", id),
  )
}

export function closeTraderPositionApi(id: string, data: ClosePositionRequest) {
  return request.post<ClosePositionPayload>(
    Api.TraderClosePosition.replace("{id}", id),
    data,
  )
}

export function getEquityHistoryApi(params?: EquityHistoryQuery) {
  return request.get<EquityHistoryPointPayload[]>(Api.EquityHistory, { params })
}

export function getTraderGridRiskApi(id: string) {
  return request.get<GridRiskInfoPayload>(
    Api.TraderGridRisk.replace("{id}", id),
  )
}

export function getTraderStatusApi(params?: TraderQuery) {
  return request.get<TraderStatusPayload>(Api.Status, { params })
}

export function getTraderAccountApi(params?: TraderQuery) {
  return request.get<TraderAccountPayload>(Api.Account, { params })
}

export function getPositionsApi(params?: PositionQuery) {
  return request.get<PositionListPayload>(Api.Positions, { params })
}

export function getPositionsHistoryApi(params?: PaginationQuery) {
  return request.get<PositionListPayload>(Api.PositionsHistory, { params })
}

export function getTradesApi(params?: PaginationQuery) {
  return request.get<TradeListPayload>(Api.Trades, { params })
}

export function getOrdersApi(params?: PaginationQuery) {
  return request.get<OrderListPayload>(Api.Orders, { params })
}

export function getOrderFillsApi(id: string, params?: TraderQuery) {
  return request.get<FillListPayload>(Api.OrderFills.replace("{id}", id), {
    params,
  })
}

export function getOpenOrdersApi(params?: PaginationQuery) {
  return request.get<OrderListPayload>(Api.OpenOrders, { params })
}

export function getDecisionsApi(params?: DecisionQuery) {
  return request.get<DecisionListPayload>(Api.Decisions, { params })
}

export function getLatestDecisionsApi(params?: TraderQuery) {
  return request.get<LatestDecisionsPayload>(Api.LatestDecisions, { params })
}

export function getStatisticsApi(params?: StatisticsQuery) {
  return request.get<TraderStatisticsPayload>(Api.Statistics, { params })
}

export function getRuntimeMetricsApi(params?: RuntimeMetricsQuery) {
  return request.get<RuntimeMetricsPayload>(Api.RuntimeMetrics, { params })
}

export function getRuntimeMetricsSeriesApi(params?: RuntimeMetricsSeriesQuery) {
  return request.get<RuntimeMetricsSeriesPayload>(Api.RuntimeMetricsSeries, {
    params,
  })
}

export function getRuntimeAlertsApi(params?: RuntimeAlertsQuery) {
  return request.get<RuntimeAlertsPayload>(Api.RuntimeAlerts, { params })
}

export function getRuntimeAlertHistoryApi(params?: RuntimeAlertHistoryQuery) {
  return request.get<RuntimeAlertHistoryPayload>(Api.RuntimeAlertHistory, {
    params,
  })
}

export function getRuntimeAlertDeliveriesApi(
  params?: RuntimeAlertDeliveriesQuery,
) {
  return request.get<RuntimeAlertDeliveriesPayload>(
    Api.RuntimeAlertDeliveries,
    {
      params,
    },
  )
}

export function getRuntimeAlertControlsApi(params?: RuntimeAlertControlsQuery) {
  return request.get<RuntimeAlertControlsPayload>(Api.RuntimeAlertControls, {
    params,
  })
}

export function muteRuntimeAlertControlsApi(data: RuntimeAlertMuteRequest) {
  return request.put<RuntimeAlertMutePayload>(Api.RuntimeAlertMute, data)
}

export function unmuteRuntimeAlertControlsApi(
  data: RuntimeAlertControlTargetRequest,
) {
  return request.put<RuntimeAlertMutePayload>(Api.RuntimeAlertUnmute, data)
}

export function ackRuntimeAlertControlsApi(data: RuntimeAlertAckRequest) {
  return request.put<RuntimeAlertAckPayload>(Api.RuntimeAlertAck, data)
}

export function getRuntimeEventsApi(params?: RuntimeEventsQuery) {
  return request.get<RuntimeEventsPayload>(Api.RuntimeEvents, { params })
}

export function getRuntimeEventTypesApi(params?: RuntimeEventTypesQuery) {
  return request.get<RuntimeEventTypesPayload>(Api.RuntimeEventTypes, {
    params,
  })
}
