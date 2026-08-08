import type {
  BacktestMessagePayload,
  BacktestQueryParams,
  BacktestRunActionPayload,
  BacktestRunIdRequest,
  BacktestRunsPayload,
  BacktestStartRequest,
} from "@/types/backtest"
import request from "@/utils/request"

const Api = {
  Start: "/api/backtest/start",
  Stop: "/api/backtest/stop",
  Delete: "/api/backtest/delete",
  Runs: "/api/backtest/runs",
} as const

export function startBacktestApi(data: BacktestStartRequest) {
  return request.post<BacktestRunActionPayload>(Api.Start, data)
}

export function stopBacktestApi(data: BacktestRunIdRequest) {
  return request.post<BacktestRunActionPayload>(Api.Stop, data)
}

export function deleteBacktestApi(data: BacktestRunIdRequest) {
  return request.post<BacktestMessagePayload>(Api.Delete, data)
}

export function getBacktestRunsApi(params?: BacktestQueryParams) {
  return request.get<BacktestRunsPayload>(Api.Runs, { params })
}
