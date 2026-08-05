export type { KlinePayload } from "@/types/public"

export interface BacktestStartRequest {
  run_id?: string | null
  symbols?: string[] | null
  start_ts?: number | null
  end_ts?: number | null
  initial_balance?: number | null
  fee_bps?: number | null
  slippage_bps?: number | null
  ai_model_id?: string | null
  prompt_variant?: string | null
  leverage?: number | null
  interval?: string | null
  decision_every?: number | null
}

export interface BacktestRunIdRequest {
  run_id: string
}

export interface BacktestLabelRequest {
  run_id: string
  label: string
}

export interface BacktestQueryParams {
  run_id?: string | null
  limit?: number | null
}

export interface KlinesQuery {
  symbol: string
  interval?: string | null
  limit?: number | null
}

export interface BacktestRunActionPayload {
  run_id: string
  message: string
}

export interface BacktestMessagePayload {
  message: string
}

export interface BacktestRunConfigPayload {
  run_id?: string
  symbols?: string[]
  start_ts?: number
  end_ts?: number
  initial_balance?: number
  fee_bps?: number
  slippage_bps?: number
  ai_model_id?: string
  prompt_variant?: string
  leverage?: number
  interval?: string
  decision_every?: number
}

export interface BacktestRunSummaryPayload {
  equity_last?: number
  final_equity?: number
  initial_balance?: number
  max_drawdown_pct?: number
  total_trades?: number
  winning_trades?: number
}

export interface BacktestRunPayload {
  run_id: string
  label: string
  state: string
  last_error: string
  summary: BacktestRunSummaryPayload
  created_at: number
  updated_at: number
}

export interface BacktestStatusItemPayload extends BacktestRunPayload {
  config: BacktestRunConfigPayload
}

export interface BacktestEquityPointPayload {
  ts: number
  equity: number
  available: number
  pnl: number
  pnl_pct: number
  dd_pct: number
  cycle: number
}

export interface BacktestTradePayload {
  id: string
  ts: number
  symbol: string
  action: string
  side: string
  qty: number
  price: number
  fee: number
  realized_pnl: number
  leverage: number
  cycle: number
  liquidation: boolean
}

export interface BacktestDecisionPayload {
  id: string
  ts: number
  symbol: string
  decision: string
  confidence: number
  reason: string
  cycle: number
}

export interface BacktestMetricsItemPayload {
  run_id: string
  state: string
  initial_balance: number
  final_equity: number
  total_pnl: number
  total_pnl_pct: number
  total_trades: number
  winning_trades: number
  win_rate_pct: number
  max_drawdown_pct: number
}

export interface BacktestStatusPayload {
  status: BacktestStatusItemPayload
}

export interface BacktestRunsPayload {
  runs: BacktestRunPayload[]
  count: number
}

export interface BacktestEquityPayload {
  points: BacktestEquityPointPayload[]
  count: number
}

export interface BacktestTradesPayload {
  trades: BacktestTradePayload[]
  count: number
}

export interface BacktestTracePayload {
  trace: BacktestDecisionPayload[]
}

export interface BacktestDecisionsPayload {
  decisions: BacktestDecisionPayload[]
  count: number
}

export interface BacktestMetricsPayload {
  metrics: BacktestMetricsItemPayload
}

export interface BacktestExportPayload {
  run_id: string
  trades: BacktestTradePayload[]
  equity: BacktestEquityPointPayload[]
  exported_at: number
}
