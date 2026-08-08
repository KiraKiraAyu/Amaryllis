export interface BacktestStartRequest {
  trader_id: string
  run_id?: string | null
  start_ts?: number | null
  end_ts?: number | null
  initial_balance?: number | null
  interval?: string | null
}

export interface BacktestRunIdRequest {
  run_id: string
}

export interface BacktestQueryParams {
  run_id?: string | null
  limit?: number | null
}

export interface BacktestRunActionPayload {
  run_id: string
  message: string
}

export interface BacktestMessagePayload {
  message: string
}

export interface BacktestRunSummaryPayload {
  final_equity?: number
  initial_balance?: number
  max_drawdown_pct?: number
  total_trades?: number
  winning_trades?: number
  total_realized_pnl?: number
  max_equity?: number
}

export interface BacktestRunPayload {
  run_id: string
  state: string
  last_error: string
  summary: BacktestRunSummaryPayload
  created_at: number
  updated_at: number
}

export interface BacktestRunsPayload {
  runs: BacktestRunPayload[]
  count: number
}
