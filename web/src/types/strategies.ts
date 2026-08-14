export interface StrategyCoinSourceConfigPayload {
  source_type: string
  static_coins: string[]
  excluded_coins: string[]
  use_ai500: boolean
  ai500_limit: number
  use_oi_top: boolean
  oi_top_limit: number
  use_oi_low: boolean
  oi_low_limit: number
}

export interface StrategyKlinesConfigPayload {
  primary_timeframe: string
  primary_count: number
  longer_timeframe: string
  longer_count: number
  enable_multi_timeframe: boolean
  selected_timeframes: string[]
}

export interface StrategyIndicatorsConfigPayload {
  klines: StrategyKlinesConfigPayload
  enable_raw_klines: boolean
  enable_ema: boolean
  enable_macd: boolean
  enable_rsi: boolean
  enable_atr: boolean
  enable_boll: boolean
  enable_volume: boolean
  enable_oi: boolean
  enable_funding_rate: boolean
  quantauraos_api_key: string
  enable_quant_data: boolean
  enable_oi_ranking: boolean
  enable_netflow_ranking: boolean
  enable_price_ranking: boolean
}

export const STRATEGY_DATA_TIMEFRAMES = [
  "5m",
  "15m",
  "30m",
  "1h",
  "4h",
  "1d",
  "1w",
] as const

export type StrategyDataTimeframe = (typeof STRATEGY_DATA_TIMEFRAMES)[number]
export type StrategyDataItemType = "raw_kline" | "indicator"
export type StrategyIndicatorType = "ema" | "macd" | "rsi" | "atr" | "bollinger"

export interface StrategyDataItemPayload {
  id: string
  type: StrategyDataItemType
  timeframes: StrategyDataTimeframe[]
  count?: number
  closed_only?: boolean
  indicator?: StrategyIndicatorType
  params?: Record<string, unknown>
  output_mode?: "latest"
}

export interface StrategyDataTemplatePayload {
  schema_version: 1
  items: StrategyDataItemPayload[]
}

export interface StrategyRiskControlConfigPayload {
  max_positions: number
  leverage: number
  max_position_value_ratio: number
  max_margin_usage: number
  min_position_size: number
  min_risk_reward_ratio: number
  min_confidence: number
}

export type StrategyTpSlMode = "fixed" | "custom"

export interface StrategyTakeProfitConfigPayload {
  mode: StrategyTpSlMode
  /** Positive unrealized PnL ratio (e.g. 0.10 = +10%). */
  pnl_rate?: number | null
  custom_prompt?: string | null
}

export interface StrategyStopLossConfigPayload {
  mode: StrategyTpSlMode
  /** Negative unrealized PnL ratio (e.g. -0.05 = -5%). */
  pnl_rate?: number | null
  custom_prompt?: string | null
}

export interface StrategyTpSlConfigPayload {
  take_profit: StrategyTakeProfitConfigPayload
  stop_loss: StrategyStopLossConfigPayload
}

export interface StrategyPromptSectionsConfigPayload {
  role_definition: string
  trading_frequency: string
  entry_standards: string
  decision_process: string
}

export interface StrategyGridConfigPayload {
  symbol: string
  grid_count: number
  total_investment: number
  leverage: number
  upper_price: number
  lower_price: number
  use_atr_bounds: boolean
  atr_multiplier: number
  distribution: string
  max_drawdown_pct: number
  stop_loss_pct: number
  daily_loss_limit_pct: number
  use_maker_only: boolean
  enable_direction_adjust: boolean
  direction_bias_ratio: number
}

export interface StrategySymbolConfig {
  symbol: string
  leverage: number
  min_cost?: number | null
  max_cost?: number | null
  fixed_cost?: number | null
}

export interface StrategyConfigPayload {
  symbols?: StrategySymbolConfig[]
  strategy_type?: string
  coin_source?: StrategyCoinSourceConfigPayload
  indicators?: StrategyIndicatorsConfigPayload
  data_template?: StrategyDataTemplatePayload
  custom_prompt?: string
  risk_control?: StrategyRiskControlConfigPayload
  tp_sl?: StrategyTpSlConfigPayload
  prompt_sections?: StrategyPromptSectionsConfigPayload
  grid_config?: StrategyGridConfigPayload
  trading_symbols?: string
  max_positions?: number
  prompt_variant?: string
}

export interface StrategyDecisionPayload {
  action: string
  symbol: string
  confidence: number
  reasoning: string
}

export interface CreateStrategyRequest {
  name: string
  description?: string
  config: StrategyConfigPayload
}

export interface UpdateStrategyRequest {
  name?: string
  description?: string
  config: StrategyConfigPayload
}

export interface DuplicateStrategyRequest {
  name?: string
}

export interface PreviewPromptRequest {
  config: StrategyConfigPayload
  account_equity?: number | null
  prompt_variant?: string | null
}

export interface StrategyTestRunRequest {
  config: StrategyConfigPayload
  prompt_variant?: string | null
  ai_model_id?: string | null
  run_real_ai?: boolean | null
}

export interface StrategyPayload {
  id: string
  name: string
  description: string
  author_email: string
  is_active: boolean
  config: StrategyConfigPayload
  created_at: string
  updated_at: string
}

export interface StrategyListPayload {
  strategies: StrategyPayload[]
  count: number
}

export interface StrategyCreatedPayload {
  id: string
  message: string
}

export interface StrategyMessagePayload {
  message: string
}

export interface PreviewPromptPayload {
  system_prompt: string
  prompt_variant: string
}

export interface StrategyTestRunPayload {
  system_prompt: string
  user_prompt: string
  prompt_variant: string
  ai_model_id: string
  ai_response: string
  decisions: StrategyDecisionPayload[]
  reasoning: string
  duration_ms: number
  used_real_ai: boolean
}
