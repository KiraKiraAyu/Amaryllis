import type {
  CreateStrategyRequest,
  DuplicateStrategyRequest,
  PreviewPromptPayload,
  PreviewPromptRequest,
  StrategyCreatedPayload,
  StrategyListPayload,
  StrategyMessagePayload,
  StrategyPayload,
  StrategyTestRunPayload,
  StrategyTestRunRequest,
  UpdateStrategyRequest,
} from "@/types/strategies"
import type { StrategyPositionListPayload } from "@/types/trading"
import request from "@/utils/request"

const Api = {
  Root: "/api/strategies",
  Active: "/api/strategies/active",
  PreviewPrompt: "/api/strategies/preview-prompt",
  TestRun: "/api/strategies/test-run",
  Detail: "/api/strategies/{id}",
  Activate: "/api/strategies/{id}/activate",
  Duplicate: "/api/strategies/{id}/duplicate",
  Positions: "/api/strategies/{id}/positions",
} as const

export function getStrategiesApi() {
  return request.get<StrategyListPayload>(Api.Root)
}

export function createStrategyApi(data: CreateStrategyRequest) {
  return request.post<StrategyCreatedPayload>(Api.Root, data)
}

export function getActiveStrategyApi() {
  return request.get<StrategyPayload>(Api.Active)
}

export function previewStrategyPromptApi(data: PreviewPromptRequest) {
  return request.post<PreviewPromptPayload>(Api.PreviewPrompt, data)
}

export function strategyTestRunApi(data: StrategyTestRunRequest) {
  return request.post<StrategyTestRunPayload>(Api.TestRun, data)
}

export function getStrategyApi(id: string) {
  return request.get<StrategyPayload>(Api.Detail.replace("{id}", id))
}

export function updateStrategyApi(id: string, data: UpdateStrategyRequest) {
  return request.put<StrategyMessagePayload>(
    Api.Detail.replace("{id}", id),
    data,
  )
}

export function deleteStrategyApi(id: string) {
  return request.delete<StrategyMessagePayload>(Api.Detail.replace("{id}", id))
}

export function activateStrategyApi(id: string) {
  return request.post<StrategyMessagePayload>(Api.Activate.replace("{id}", id))
}

export function duplicateStrategyApi(
  id: string,
  data: DuplicateStrategyRequest,
) {
  return request.post<StrategyCreatedPayload>(
    Api.Duplicate.replace("{id}", id),
    data,
  )
}

export function getStrategyPositionsApi(id: string) {
  return request.get<StrategyPositionListPayload>(
    Api.Positions.replace("{id}", id),
  )
}
