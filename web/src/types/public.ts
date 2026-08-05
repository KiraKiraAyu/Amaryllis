export interface EquityHistoryQuery {
  trader_id?: string | null
}

export interface EquityHistoryBatchRequest {
  trader_ids?: string[] | null
  hours?: number | null
}

export interface SymbolsQuery {
  exchange?: string | null
}

export interface KlinesQuery {
  symbol: string
  interval?: string | null
  limit?: number | null
  exchange?: string | null
}

export interface CryptoConfigPayload {
  transport_encryption: boolean
}

export interface CryptoPublicKeyPayload {
  transport_encryption: boolean
  public_key: string
}

export interface ServerIpPayload {
  public_ip: string
  message: string
}

export interface EquityHistoryPointPayload {
  timestamp: string
  total_equity: number
  available_balance: number
  total_pnl: number
  total_pnl_pct: number
  position_count: number
  margin_used_pct: number
  balance: number
}

export interface EquityHistoryBatchPayload {
  histories: Record<string, EquityHistoryPointPayload[]>
  errors: Record<string, string>
}

export interface PublicTraderConfigPayload {
  trader_id: string
  trader_name: string
  ai_model: string
  exchange_id: string
  strategy_id: string
  is_cross_margin: boolean
  scan_interval_minutes: number
  initial_balance: number
  is_running: boolean
  custom_prompt: string
  override_base_prompt: boolean
  system_prompt_template: string
  use_ai500: boolean
  use_oi_top: boolean
}

export interface ExchangeSymbolPayload {
  symbol: string
  name: string
  category: string
}

export interface KlinePayload {
  openTime: number
  open: number
  high: number
  low: number
  close: number
  volume: number
  quoteVolume: number
  closeTime: number
}

export interface ExchangeSymbolsPayload {
  exchange: string
  symbols: ExchangeSymbolPayload[]
  count: number
}

export interface SupportedProviderTypePayload {
  providerType: string
  name: string
}

export interface SupportedExchangePayload {
  id: string
  name: string
  type: string
}
