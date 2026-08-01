export interface CreateTraderRequest {
  name: string
  ai_model_id: string
  exchange_id: string
  strategy_id?: string
  initial_balance?: number
  scan_interval_minutes?: number
  is_cross_margin?: boolean | null
  show_in_competition?: boolean | null
  btc_eth_leverage?: number
  altcoin_leverage?: number
  trading_symbols?: string
  use_ai500?: boolean
  use_oi_top?: boolean
  custom_prompt?: string
  override_base_prompt?: boolean
  system_prompt_template?: string
}

export interface UpdateTraderRequest {
  name?: string | null
  ai_model_id?: string | null
  exchange_id?: string | null
  strategy_id?: string | null
  initial_balance?: number | null
  scan_interval_minutes?: number | null
  is_cross_margin?: boolean | null
  show_in_competition?: boolean | null
  btc_eth_leverage?: number | null
  altcoin_leverage?: number | null
  trading_symbols?: string | null
  use_ai500?: boolean | null
  use_oi_top?: boolean | null
  custom_prompt?: string | null
  override_base_prompt?: boolean | null
  system_prompt_template?: string | null
}

export interface UpdatePromptRequest {
  custom_prompt?: string
  override_base_prompt?: boolean
}

export interface ToggleCompetitionRequest {
  show_in_competition: boolean
}

export interface ClosePositionRequest {
  symbol: string
  side: string
  local_only?: boolean
}

export interface TraderQuery {
  trader_id?: string | null
}

export interface PaginationQuery {
  trader_id?: string | null
  limit?: number | null
  offset?: number | null
}

export interface PositionQuery {
  trader_id?: string | null
  status?: string | null
}

export interface DecisionQuery {
  trader_id?: string | null
  limit?: number | null
  offset?: number | null
  symbol?: string | null
}

export interface StatisticsQuery {
  trader_id?: string | null
  days?: number | null
}

export interface RuntimeMetricsQuery {
  trader_id?: string | null
  window_hours?: number | null
}

export interface RuntimeMetricsSeriesQuery {
  trader_id?: string | null
  window_hours?: number | null
  bucket_minutes?: number | null
}

export interface RuntimeAlertsQuery {
  trader_id?: string | null
  window_hours?: number | null
  open_market_fallback_rate_max_pct?: number | null
  replace_throttle_rate_max_pct?: number | null
  stale_reconcile_terminal_rate_max_pct?: number | null
  persist_min_interval_secs?: number | null
}

export interface RuntimeAlertHistoryQuery {
  trader_id?: string | null
  window_hours?: number | null
  limit?: number | null
  offset?: number | null
  breached_only?: boolean | null
  severity?: string | null
}

export interface RuntimeAlertDeliveriesQuery {
  trader_id?: string | null
  window_hours?: number | null
  limit?: number | null
  offset?: number | null
  success?: boolean | null
  destination?: string | null
}

export interface RuntimeAlertControlsQuery {
  trader_id?: string | null
}

export interface RuntimeAlertControlTargetRequest {
  trader_id?: string | null
}

export interface RuntimeAlertMuteRequest {
  trader_id?: string | null
  mute_minutes?: number | null
  mute_until?: number | null
  reason?: string | null
}

export interface RuntimeAlertAckRequest {
  trader_id?: string | null
  note?: string | null
}

export interface RuntimeEventsQuery {
  trader_id?: string | null
  window_hours?: number | null
  limit?: number | null
  offset?: number | null
  event_type?: string | null
  risk_level?: string | null
  correlation_id?: string | null
}

export interface RuntimeEventTypesQuery {
  trader_id?: string | null
  window_hours?: number | null
}

export interface TraderPayload {
  id: string
  name: string
  ai_model_id: string
  exchange_id: string
  strategy_id: string
  initial_balance: number
  scan_interval_minutes: number
  is_running: boolean
  is_cross_margin: boolean
  show_in_competition: boolean
  btc_eth_leverage: number
  altcoin_leverage: number
  trading_symbols: string
  use_ai500: boolean
  use_oi_top: boolean
  custom_prompt: string
  override_base_prompt: boolean
  system_prompt_template: string
  created_at: number
  updated_at: number
}

export interface TraderListPayload {
  traders: TraderPayload[]
  count: number
}

export interface TraderCreatedPayload {
  id: string
  message: string
}

export interface TraderMessagePayload {
  message: string
}

export interface TraderAccountPayload {
  trader_id: string
  total_balance: number
  available_balance: number
  used_margin: number
  unrealized_pnl: number
  realized_pnl: number
  currency: string
  snapshot_at: number
}

export interface TraderBalanceSyncPayload {
  message: string
  mode: string
  account: TraderAccountPayload
}

export interface ClosePositionPayload {
  message: string
  mode: string
  order_id: string
  symbol: string
  side: string
}

export interface SymbolConcentrationPayload {
  symbol: string
  notional: number
  weight_pct: number
}

export interface GridRiskInfoPayload {
  trader_id: string
  total_notional: number
  symbol_concentration: SymbolConcentrationPayload[]
}

export interface RuntimeEnginePayload {
  trader_id: string
  exchange_id: string
  ai_model_id: string
  started_at: number
  updated_at: number
  is_running: boolean
  last_error: string | null
}

export interface TraderStatusPayload {
  trader_id: string
  is_running: boolean
  open_positions: number
  open_orders: number
  last_updated: number
  runtime_engine: RuntimeEnginePayload | null
}

export interface PositionPayload {
  id: string
  trader_id: string
  symbol: string
  side: string
  quantity: number
  entry_price: number
  mark_price: number
  liquidation_price: number
  leverage: number
  margin_mode: string
  unrealized_pnl: number
  realized_pnl: number
  status: string
  opened_at: number
  closed_at: number | null
  updated_at: number
}

export interface PositionListPayload {
  trader_id: string
  items: PositionPayload[]
  count: number
}

export interface DecisionPayload {
  id: string
  symbol: string
  timeframe: string
  decision: string
  confidence: number
  reason: string
  payload_json: string
  created_at: number
}

export interface DecisionListPayload {
  trader_id: string
  items: DecisionPayload[]
  count: number
  limit: number
  offset: number
  symbol: string | null
}

export interface LatestDecisionsPayload {
  trader_id: string
  items: DecisionPayload[]
  count: number
}

export interface TradePayload {
  id: string
  symbol: string
  side: string
  entry_price: number
  exit_price: number
  quantity: number
  realized_pnl: number
  fees: number
  roi_pct: number
  opened_at: number
  closed_at: number
}

export interface TradeListPayload {
  trader_id: string
  items: TradePayload[]
  count: number
  limit: number
  offset: number
}

export interface OrderPayload {
  id: string
  exchange_order_id: string
  client_order_id: string
  symbol: string
  side: string
  position_side: string
  order_type: string
  status: string
  price: number
  quantity: number
  filled_quantity: number
  avg_fill_price: number | null
  reduce_only: boolean
  time_in_force: string
  placed_at: number
  updated_at: number
  closed_at: number | null
}

export interface OrderListPayload {
  trader_id: string
  items: OrderPayload[]
  count: number
  limit: number
  offset: number
}

export interface FillPayload {
  id: string
  exchange_trade_id: string
  symbol: string
  side: string
  price: number
  quantity: number
  fee: number
  fee_asset: string
  realized_pnl: number
  executed_at: number
}

export interface FillListPayload {
  trader_id: string
  order_id: string
  items: FillPayload[]
  count: number
}

export interface RuntimeEventPayload {
  id: string
  event_type: string
  symbol: string
  side: string
  risk_level: string
  trigger_source: string
  action_taken: string
  correlation_id: string
  payload: Record<string, unknown>
  created_at: number
}

export interface RuntimeEventsFilterPayload {
  event_type: string
  risk_level: string
  correlation_id: string
}

export interface RuntimeEventsPayload {
  trader_id: string
  window_hours: number
  from_ts: number
  limit: number
  offset: number
  filters: RuntimeEventsFilterPayload
  total: number
  items: RuntimeEventPayload[]
}

export interface RuntimeEventTypePayload {
  event_type: string
  count: number
  description: string
  canonical: boolean
}

export interface RuntimeEventTypesPayload {
  trader_id: string
  window_hours: number
  from_ts: number
  items: RuntimeEventTypePayload[]
}

export interface RuntimeMetricTotalsPayload {
  runtime_events: number
  replace_succeeded: number
  replace_throttled: number
  replace_market_fallback: number
  open_market_fallback: number
  open_submitted: number
  stale_reconcile_terminal: number
  stale_reconcile_pending: number
  medium_risk_open_skips: number
  live_risk_snapshots: number
}

export interface RuntimeMetricRatesPayload {
  replace_throttle_rate: number
  replace_market_fallback_rate: number
  open_market_fallback_rate: number
  stale_reconcile_terminal_rate: number
}

export interface RiskLevelCountPayload {
  risk_level: string
  count: number
}

export interface RuntimeMetricsPayload {
  trader_id: string
  window_hours: number
  from_ts: number
  totals: RuntimeMetricTotalsPayload
  rates_pct: RuntimeMetricRatesPayload
  risk_level_distribution: RiskLevelCountPayload[]
}

export interface RuntimeMetricsSeriesBucketPayload {
  bucket_from_ts: number
  bucket_to_ts: number
  totals: RuntimeMetricTotalsPayload
  rates_pct: RuntimeMetricRatesPayload
}

export interface RuntimeMetricsSeriesPayload {
  trader_id: string
  window_hours: number
  from_ts: number
  bucket_minutes: number
  bucket_secs: number
  items: RuntimeMetricsSeriesBucketPayload[]
}

export interface RuntimeAlertThresholdsPayload {
  open_market_fallback_rate_max: number
  replace_throttle_rate_max: number
  stale_reconcile_terminal_rate_max: number
}

export interface RuntimeAlertRatesPayload {
  open_market_fallback_rate: number
  replace_throttle_rate: number
  stale_reconcile_terminal_rate: number
}

export interface RuntimeAlertItemPayload {
  key: string
  label: string
  rate_pct: number
  max_pct: number
  breached: boolean
}

export interface RuntimeAlertTotalsPayload {
  replace_succeeded: number
  replace_throttled: number
  open_market_fallback: number
  open_submitted: number
  stale_reconcile_terminal: number
  stale_reconcile_pending: number
}

export interface RuntimeAlertStatePayload {
  breached: boolean
  severity: string
  muted: boolean
  acked_at: number
}

export interface RuntimeAlertControlsPayload {
  trader_id: string
  is_muted: boolean
  muted_until: number
  mute_reason: string
  acked_at: number
  acked_by: string
  ack_note: string
  updated_at: number
  created_at: number
}

export interface RuntimeAlertNotificationPayload {
  channel: string
  suppressed: boolean
  reason: string
  attempts: number
  max_attempts: number
  success: boolean
  status: number
  error: string
}

export interface RuntimeAlertsPayload {
  trader_id: string
  window_hours: number
  from_ts: number
  persist_min_interval_secs: number
  thresholds_pct: RuntimeAlertThresholdsPayload
  rates_pct: RuntimeAlertRatesPayload
  totals: RuntimeAlertTotalsPayload
  alerts: RuntimeAlertItemPayload[]
  alert_state: RuntimeAlertStatePayload
  controls: RuntimeAlertControlsPayload
  alert_history_id: string
  notification: RuntimeAlertNotificationPayload
}

export interface RuntimeAlertHistoryItemPayload {
  id: string
  window_hours: number
  thresholds_pct: RuntimeAlertThresholdsPayload
  rates_pct: RuntimeAlertRatesPayload
  alerts: RuntimeAlertItemPayload[]
  breached: boolean
  severity: string
  created_at: number
}

export interface RuntimeAlertHistoryFiltersPayload {
  breached_only: boolean | null
  severity: string
}

export interface RuntimeAlertHistoryPayload {
  trader_id: string
  window_hours: number
  from_ts: number
  limit: number
  offset: number
  filters: RuntimeAlertHistoryFiltersPayload
  total: number
  items: RuntimeAlertHistoryItemPayload[]
}

export interface RuntimeAlertDeliveryLogPayload {
  id: string
  alert_history_id: string
  destination: string
  endpoint: string
  response_status: number
  response_body: string
  attempt: number
  max_attempts: number
  success: boolean
  error_message: string
  latency_ms: number
  created_at: number
}

export interface RuntimeAlertDeliveriesFiltersPayload {
  success: boolean | null
  destination: string
}

export interface RuntimeAlertDeliveriesPayload {
  trader_id: string
  window_hours: number
  from_ts: number
  limit: number
  offset: number
  filters: RuntimeAlertDeliveriesFiltersPayload
  total: number
  items: RuntimeAlertDeliveryLogPayload[]
}

export interface RuntimeAlertMutePayload {
  message: string
  trader_id: string
  is_muted: boolean
  muted_until: number
  mute_reason: string
}

export interface RuntimeAlertAckPayload {
  message: string
  trader_id: string
  acked_at: number
  acked_by: string
  ack_note: string
}

export interface TraderStatisticsPayload {
  trader_id: string
  period_days: number
  total_trades: number
  winning_trades: number
  win_rate_pct: number
  total_realized_pnl: number
  total_fees: number
  net_pnl: number
  avg_roi_pct: number
  open_positions: number
}
