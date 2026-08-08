import type { BacktestRunPayload } from "@/types/backtest"

export interface BacktestConfig {
  interval: string
  startDate: string
  endDate: string
  initial_balance: number
}

export type BacktestRun = BacktestRunPayload

export interface BacktestLiveProgress {
  run_id: string
  state: string
  bar_index: number
  total_bars: number
  equity: number
}
