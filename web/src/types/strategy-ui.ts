import type {
  StrategyConfigPayload,
  StrategyDecisionPayload,
  StrategyPayload,
  StrategyTestRunPayload,
} from "@/types/strategies"

export type StrategyConfig = StrategyConfigPayload
export type EditableStrategy = StrategyPayload
export type StrategyDecision = StrategyDecisionPayload
export type StrategyTestResult = StrategyTestRunPayload

export interface StrategyConfigFormFields {
  trading_symbols?: string
  max_positions?: number
  prompt_variant?: string
}

export interface StrategyPromptPreviewModel {
  system: string
  variant: string
}
