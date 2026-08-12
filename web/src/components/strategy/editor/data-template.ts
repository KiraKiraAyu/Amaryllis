import type {
  StrategyConfigPayload,
  StrategyDataItemPayload,
  StrategyDataTemplatePayload,
  StrategyIndicatorType,
} from "@/types/strategies"
import { STRATEGY_DATA_TIMEFRAMES } from "@/types/strategies"

export const timeframeOptions = [...STRATEGY_DATA_TIMEFRAMES]

export const indicatorOptions: {
  label: string
  value: StrategyIndicatorType
}[] = [
  { label: "EMA", value: "ema" },
  { label: "MACD", value: "macd" },
  { label: "RSI", value: "rsi" },
  { label: "ATR", value: "atr" },
  { label: "Bollinger", value: "bollinger" },
]

export function defaultDataTemplate(): StrategyDataTemplatePayload {
  return {
    schema_version: 1,
    items: [
      {
        id: "primary-kline",
        type: "raw_kline",
        timeframes: ["5m"],
        count: 100,
        closed_only: true,
        output_mode: "latest",
      },
    ],
  }
}

export function getOrCreateDataTemplate(
  config: StrategyConfigPayload,
): StrategyDataTemplatePayload {
  if (!config.data_template) config.data_template = defaultDataTemplate()
  return config.data_template
}

export function dataItemLabel(item: StrategyDataItemPayload): string {
  if (item.type === "raw_kline") return "Kline"
  return (
    indicatorOptions.find((option) => option.value === item.indicator)?.label ??
    "Indicator"
  )
}

export function computeItemError(item: StrategyDataItemPayload): string {
  if (!item.timeframes.length) return "Needs at least one timeframe."
  if (!item.timeframes.every((tf) => timeframeOptions.includes(tf)))
    return "Has an unsupported timeframe."
  if (
    item.type === "raw_kline" &&
    (!item.count || item.count < 1 || item.count > 1500)
  )
    return "Count must be between 1 and 1500."
  if (item.type === "indicator") {
    const period = Number(item.params?.period ?? 0)
    if (period && period > 1500) return "Period must be at most 1500."
  }
  return ""
}

export function computeDataTemplateError(
  items: StrategyDataItemPayload[],
): string {
  if (!items.length) return "Add at least one data item."
  for (const item of items) {
    const err = computeItemError(item)
    if (err) return `${dataItemLabel(item)}: ${err}`
  }
  return ""
}
