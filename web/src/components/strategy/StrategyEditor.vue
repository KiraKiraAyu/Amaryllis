<script setup lang="ts">
import Card from "primevue/card"
import Button from "primevue/button"
import InputText from "primevue/inputtext"
import InputNumber from "primevue/inputnumber"
import Select from "primevue/select"
import MultiSelect from "primevue/multiselect"
import Textarea from "primevue/textarea"
import { ref, computed, watch, nextTick } from "vue"
import type { EditableStrategy } from "@/types/strategy-ui"
import type {
  StrategyTpSlConfigPayload,
  StrategyTpSlMode,
  StrategyTakeProfitConfigPayload,
  StrategyStopLossConfigPayload,
  StrategyDataTemplatePayload,
  StrategyDataItemPayload,
  StrategyIndicatorType,
} from "@/types/strategies"
import { STRATEGY_DATA_TIMEFRAMES } from "@/types/strategies"

const selected = defineModel<EditableStrategy>({ required: true })

defineProps<{
  saving: boolean
  duplicating: boolean
  testRunLoading?: boolean
}>()

const emit = defineEmits<{
  save: []
  cancel: []
  test: []
}>()

const config = computed(() => {
  if (!selected.value.config) selected.value.config = {}
  return selected.value.config
})

const timeframeOptions = [...STRATEGY_DATA_TIMEFRAMES]
const indicatorOptions = [
  { label: "EMA", value: "ema" },
  { label: "MACD", value: "macd" },
  { label: "RSI", value: "rsi" },
  { label: "ATR", value: "atr" },
  { label: "Bollinger", value: "bollinger" },
]

const defaultDataTemplate = (): StrategyDataTemplatePayload => ({
  schema_version: 1,
  items: [{
    id: "primary-kline",
    type: "raw_kline",
    timeframes: ["5m"],
    count: 100,
    closed_only: true,
    output_mode: "latest",
  }],
})

const dataTemplate = computed<StrategyDataTemplatePayload>(() => {
  if (!config.value.data_template) config.value.data_template = defaultDataTemplate()
  return config.value.data_template
})
const dataItems = computed(() => dataTemplate.value.items)
const rawKlineItems = computed(() => dataItems.value.filter(item => item.type === "raw_kline"))
const manuallyEditedIndicatorTimeframes = ref(new Set<string>())
const indicatorPickerValue = ref<StrategyIndicatorType | null>(null)
const indicatorPicker = ref<{ show: () => void } | null>(null)

function dataItemLabel(item: StrategyDataItemPayload) {
  if (item.type === "raw_kline") return "Kline"
  return indicatorOptions.find(option => option.value === item.indicator)?.label ?? "Indicator"
}

const dataTemplateError = computed(() => {
  const items = dataItems.value
  if (!items.length) {
    return "Add at least one data item."
  }
  for (const item of items) {
    if (!item.id.trim()) return "Every data item needs an id."
    if (!item.timeframes.length) return `${dataItemLabel(item)} needs at least one timeframe.`
    if (!item.timeframes.every(tf => timeframeOptions.includes(tf))) return `${dataItemLabel(item)} has an unsupported timeframe.`
    if (item.type === "raw_kline" && (!item.count || item.count < 1 || item.count > 1500)) return `${dataItemLabel(item)} count must be between 1 and 1500.`
    if (item.type === "indicator") {
      const period = Number(item.params?.period ?? 0)
      if (period && period > 1500) return `${dataItemLabel(item)} period must be at most 1500.`
    }
  }
  const ids = new Set<string>()
  if (items.some(item => ids.has(item.id) || !ids.add(item.id))) return "Data item ids must be unique."
  return ""
})

function addKlineItem() {
  if (rawKlineItems.value.length >= 1) return
  const id = `kline-${Date.now()}`
  dataItems.value.push({ id, type: "raw_kline", timeframes: ["5m"], count: 100, closed_only: true, output_mode: "latest" })
}

function addIndicatorItem(indicator: StrategyIndicatorType) {
  const id = `indicator-${Date.now()}`
  const kline = rawKlineItems.value[0]
  const timeframes: StrategyDataItemPayload["timeframes"] = kline ? [...kline.timeframes] : ["5m"]
  dataItems.value.push({ id, type: "indicator", timeframes, indicator, params: { period: 14 }, output_mode: "latest" })
}

async function openIndicatorPicker() {
  indicatorPickerValue.value = null
  await nextTick()
  indicatorPicker.value?.show()
}

function selectIndicator(value: StrategyIndicatorType | null) {
  if (!value) return
  addIndicatorItem(value)
  indicatorPickerValue.value = null
}

function removeDataItem(index: number) {
  dataItems.value.splice(index, 1)
}

function indicatorPeriod(item: StrategyDataItemPayload) {
  return Number(item.params?.period ?? 14)
}

function setIndicatorPeriod(item: StrategyDataItemPayload, value: number | null) {
  item.params = { ...(item.params ?? {}), period: value ?? 14 }
}

function setIndicatorTimeframes(item: StrategyDataItemPayload, value: StrategyDataItemPayload["timeframes"] | null) {
  manuallyEditedIndicatorTimeframes.value.add(item.id)
  item.timeframes = value ?? []
}

watch(
  () => rawKlineItems.value[0]?.timeframes,
  (timeframes) => {
    if (!timeframes) return
    for (const item of dataItems.value) {
      if (item.type === "indicator" && !manuallyEditedIndicatorTimeframes.value.has(item.id)) {
        item.timeframes = [...timeframes]
      }
    }
  },
  { deep: true },
)

watch(() => selected.value.id, () => {
  manuallyEditedIndicatorTimeframes.value.clear()
})

const symbols = computed({
  get() {
    if (!config.value.symbols) {
      config.value.symbols = []
    }
    return config.value.symbols
  },
  set(val) {
    config.value.symbols = val
  }
})

const newSymbolName = ref("")
const isAddingSymbol = ref(false)

const vFocus = {
  mounted: (el: HTMLInputElement) => {
    if (el.tagName === 'INPUT') {
      el.focus()
    } else {
      el.querySelector('input')?.focus()
    }
  }
}

function confirmAddSymbol() {
  const sym = newSymbolName.value.trim().toUpperCase()
  if (!sym) {
    isAddingSymbol.value = false
    return
  }
  if (symbols.value.some(s => s.symbol === sym)) {
    return
  }
  symbols.value.push({
    symbol: sym,
    leverage: 5,
    min_cost: 20,
    max_cost: 1000,
    fixed_cost: null
  })
  newSymbolName.value = ""
  isAddingSymbol.value = false
}

function cancelAddSymbol() {
  newSymbolName.value = ""
  isAddingSymbol.value = false
}

function removeSymbol(index: number) {
  symbols.value.splice(index, 1)
}

function getCostMode(item: any) {
  return item.fixed_cost != null ? 'fixed' : 'dynamic'
}

function changeCostMode(index: number, mode: 'fixed' | 'dynamic') {
  const item = symbols.value[index]
  if (mode === 'fixed') {
    item.fixed_cost = 50.0
    item.min_cost = null
    item.max_cost = null
  } else {
    item.fixed_cost = null
    item.min_cost = 20.0
    item.max_cost = 1000.0
  }
}

const takeProfitMode = ref<StrategyTpSlMode>('fixed')
const stopLossMode = ref<StrategyTpSlMode>('fixed')
// Both values are positive percentages in the UI; stop-loss is stored as a negative ratio.
const takeProfitPnlRate = ref<number | null>(null)
const stopLossPnlRate = ref<number | null>(null)
const takeProfitPrompt = ref('')
const stopLossPrompt = ref('')
let tpSlInitialized = false

function emptyTakeProfitRule(): StrategyTakeProfitConfigPayload {
  return {
    mode: 'fixed',
    pnl_rate: null,
    custom_prompt: null
  }
}

function emptyStopLossRule(): StrategyStopLossConfigPayload {
  return {
    mode: 'fixed',
    pnl_rate: null,
    custom_prompt: null
  }
}

function emptyTpSlConfig(): StrategyTpSlConfigPayload {
  return {
    take_profit: emptyTakeProfitRule(),
    stop_loss: emptyStopLossRule()
  }
}

// Initialize once from existing config (waits for async data if needed)
watch(() => config.value.tp_sl, (tpSl) => {
  if (tpSlInitialized) return
  if (!tpSl?.take_profit || !tpSl.stop_loss) {
    config.value.tp_sl = emptyTpSlConfig()
    tpSl = config.value.tp_sl
  }
  takeProfitMode.value = tpSl.take_profit.mode
  stopLossMode.value = tpSl.stop_loss.mode
  takeProfitPnlRate.value = tpSl.take_profit.pnl_rate != null
    ? Math.abs(tpSl.take_profit.pnl_rate * 100)
    : null
  stopLossPnlRate.value = tpSl.stop_loss.pnl_rate != null
    ? Math.abs(tpSl.stop_loss.pnl_rate * 100)
    : null
  takeProfitPrompt.value = tpSl.take_profit.custom_prompt ?? ''
  stopLossPrompt.value = tpSl.stop_loss.custom_prompt ?? ''
  tpSlInitialized = true
}, { immediate: true })

// Sync each rule back to config — only after initialization.
watch([
  takeProfitMode,
  stopLossMode,
  takeProfitPnlRate,
  stopLossPnlRate,
  takeProfitPrompt,
  stopLossPrompt
], () => {
  if (!tpSlInitialized) return
  let tpSlConfig = config.value.tp_sl
  if (!tpSlConfig?.take_profit || !tpSlConfig.stop_loss) {
    tpSlConfig = emptyTpSlConfig()
    config.value.tp_sl = tpSlConfig
  }
  tpSlConfig.take_profit = {
    mode: takeProfitMode.value,
    pnl_rate: takeProfitMode.value === 'fixed' && takeProfitPnlRate.value != null
      ? takeProfitPnlRate.value / 100
      : null,
    custom_prompt: takeProfitMode.value === 'custom' ? takeProfitPrompt.value : null
  }
  tpSlConfig.stop_loss = {
    mode: stopLossMode.value,
    pnl_rate: stopLossMode.value === 'fixed' && stopLossPnlRate.value != null
      ? -(stopLossPnlRate.value / 100)
      : null,
    custom_prompt: stopLossMode.value === 'custom' ? stopLossPrompt.value : null
  }
})
</script>

<template>
  <Card class="border border-surface-200 dark:border-surface-800 bg-surface-0 dark:bg-surface-900 shadow-none!">
    <template #content>
      <div class="flex items-center gap-3 mb-6">
        <h2 class="font-bold text-xl text-surface-900 dark:text-white flex-1 truncate">Edit Strategy</h2>
      </div>

      <!-- Strategy Info -->
      <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 mb-6">
        <div class="flex flex-col gap-1.5">
          <label class="text-xs font-bold text-surface-500">Strategy Name</label>
          <InputText v-model="selected.name" placeholder="Strategy name" class="h-10 rounded-xl" />
        </div>
        <div class="flex flex-col gap-1.5">
          <label class="text-xs font-bold text-surface-500">Description</label>
          <InputText v-model="selected.description" placeholder="Brief description" class="h-10 rounded-xl" />
        </div>
      </div>

      <!-- Global Strategy parameters -->
      <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 mb-6">
        <div class="flex flex-col gap-1.5">
          <label class="text-xs font-bold text-surface-500">Max Positions</label>
          <InputNumber
            v-model="config.max_positions"
            :min="1"
            :max="20"
            showButtons
            class="h-10 rounded-xl"
          />
        </div>
        <div class="flex flex-col gap-1.5">
          <label class="text-xs font-bold text-surface-500">Prompt Variant</label>
          <Select
            v-model="config.prompt_variant"
            :options="['balanced', 'aggressive', 'conservative']"
            placeholder="Select variant"
            class="h-10 rounded-xl flex items-center"
          />
        </div>
      </div>

      <!-- Strategy data template -->
      <section class="flex flex-col gap-3 mb-6">
        <div class="flex items-center">
          <label class="text-xs font-bold text-surface-500">Strategy Data</label>
        </div>
        <div class="border border-surface-200 dark:border-surface-800 rounded-2xl p-4 flex flex-col gap-3">
          <div v-if="dataItems.length === 0" class="text-sm text-rose-500">{{ dataTemplateError }}</div>
          <div class="flex flex-wrap gap-2">
            <Button icon="pi pi-plus" label="Add Kline" size="small" severity="secondary" :disabled="rawKlineItems.length >= 1" @click="addKlineItem" />
            <div class="relative">
              <Button icon="pi pi-plus" label="Add Indicator" size="small" severity="secondary" @click="openIndicatorPicker" />
              <Select
                ref="indicatorPicker"
                v-model="indicatorPickerValue"
                :options="indicatorOptions"
                optionLabel="label"
                optionValue="value"
                filter
                autoFilterFocus
                filterPlaceholder="Search indicator"
                class="indicator-picker-select"
              >
                <template #option="{ option }">
                  <span class="p-select-option-label indicator-picker-option" @mousedown.stop.prevent="selectIndicator(option.value)">
                    {{ option.label }}
                  </span>
                </template>
              </Select>
            </div>
          </div>

          <div
            v-for="(item, index) in dataItems"
            :key="item.id || index"
            :class="[
              'border border-surface-200 dark:border-surface-800 rounded-xl p-3 grid grid-cols-1 gap-3 items-center',
              item.type === 'raw_kline'
                ? 'lg:grid-cols-[7rem_minmax(18rem,1fr)_minmax(13rem,auto)_auto_auto]'
                : 'lg:grid-cols-[7rem_minmax(18rem,1fr)_minmax(11rem,auto)_auto]',
            ]"
          >
            <div class="flex items-center gap-2 min-w-0">
              <span class="truncate text-sm font-semibold text-surface-700 dark:text-surface-200">{{ dataItemLabel(item) }}</span>
            </div>

            <template v-if="item.type === 'raw_kline'">
              <div class="flex min-w-0 items-center gap-2">
                <label class="shrink-0 text-xs text-surface-500">Timeframes</label>
                <MultiSelect v-model="item.timeframes" :options="timeframeOptions" display="chip" :maxSelectedLabels="3" class="min-h-9 min-w-0 flex-1 rounded-lg" />
              </div>
              <div class="flex items-center gap-2">
                <label class="shrink-0 text-xs text-surface-500">Kline count</label>
                <InputNumber v-model="item.count" :min="1" :max="1500" showButtons class="h-9 min-w-24 flex-1 rounded-lg" />
              </div>
              <label class="flex items-center gap-2 whitespace-nowrap text-xs text-surface-500">
                <input v-model="item.closed_only" type="checkbox" class="accent-primary" /> Closed candles only
              </label>
            </template>

            <template v-else>
              <div class="flex min-w-0 items-center gap-2">
                <label class="shrink-0 text-xs text-surface-500">Timeframes</label>
                <MultiSelect :modelValue="item.timeframes" @update:modelValue="setIndicatorTimeframes(item, $event)" :options="timeframeOptions" display="chip" :maxSelectedLabels="2" class="min-h-9 min-w-0 flex-1 rounded-lg" />
              </div>
              <div class="flex items-center gap-2">
                <label class="shrink-0 text-xs text-surface-500">Period</label>
                <InputNumber :modelValue="indicatorPeriod(item)" @update:modelValue="setIndicatorPeriod(item, $event)" :min="1" :max="1500" showButtons class="h-9 min-w-24 flex-1 rounded-lg" />
              </div>
            </template>

            <Button icon="pi pi-trash" severity="danger" text rounded aria-label="Remove data item" @click="removeDataItem(index)" />
          </div>
          <p v-if="dataTemplateError" class="text-xs text-rose-500">{{ dataTemplateError }}</p>
        </div>
      </section>

      <!-- Symbols Settings list -->
      <div class="flex flex-col gap-3 mb-6">
        <label class="text-xs font-bold text-surface-500">Trading Target Symbols</label>

        <div class="overflow-x-auto border border-surface-200 dark:border-surface-800 rounded-2xl">
          <table class="w-full text-left border-collapse min-w-150">
            <thead>
              <tr class="bg-surface-50 dark:bg-surface-950 border-b border-surface-200 dark:border-surface-800">
                <th class="p-3 text-xs font-bold text-surface-500 uppercase tracking-wider w-30">Symbol</th>
                <th class="p-3 text-xs font-bold text-surface-500 uppercase tracking-wider w-30">Leverage</th>
                <th class="p-3 text-xs font-bold text-surface-500 uppercase tracking-wider w-30">Cost Mode</th>
                <th class="p-3 text-xs font-bold text-surface-500 uppercase tracking-wider">Cost Settings</th>
                <th class="p-3 text-xs font-bold text-surface-500 uppercase tracking-wider w-20 text-center">Actions</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-surface-200 dark:divide-surface-800">
              <tr v-if="symbols.length === 0">
                <td colspan="5" class="p-4 text-center text-sm text-surface-400">
                  No symbols added. Add a symbol below to start trading.
                </td>
              </tr>
              <tr v-for="(item, index) in symbols" :key="item.symbol" class="hover:bg-surface-50/50 dark:hover:bg-surface-950/20">
                <!-- Symbol -->
                <td class="p-3 text-sm font-bold text-surface-900 dark:text-white font-mono">
                  {{ item.symbol }}
                </td>

                <!-- Leverage -->
                <td class="p-3">
                  <InputNumber
                    v-model="item.leverage"
                    :min="1"
                    :max="200"
                    showButtons
                    class="h-8 rounded-lg w-22.5"
                    inputClass="text-center font-mono py-1"
                  />
                </td>

                <!-- Cost Mode -->
                <td class="p-3">
                  <Select
                    :modelValue="getCostMode(item)"
                    @update:modelValue="changeCostMode(index, $event as 'fixed' | 'dynamic')"
                    :options="[
                      { label: 'Fixed', value: 'fixed' },
                      { label: 'Dynamic', value: 'dynamic' }
                    ]"
                    optionLabel="label"
                    optionValue="value"
                    class="h-8 rounded-lg w-27.5 text-xs flex items-center"
                  />
                </td>

                <!-- Cost Settings -->
                <td class="p-3">
                  <div v-if="getCostMode(item) === 'fixed'" class="flex items-center gap-1.5 max-w-37.5">
                    <span class="text-xs text-surface-400">$</span>
                    <InputNumber
                      v-model="item.fixed_cost"
                      :min="0.1"
                      :minFractionDigits="1"
                      :maxFractionDigits="2"
                      placeholder="Fixed Cost"
                      class="h-8 rounded-lg flex-1"
                      inputClass="py-1 font-mono text-sm"
                    />
                  </div>
                  <div v-else class="flex items-center gap-2 max-w-60">
                    <span class="text-xs text-surface-400">$</span>
                    <InputNumber
                      v-model="item.min_cost"
                      placeholder="Min"
                      :min="0.1"
                      :minFractionDigits="1"
                      :maxFractionDigits="2"
                      class="h-8 rounded-lg w-20"
                      inputClass="py-1 font-mono text-sm text-center"
                    />
                    <span class="text-xs text-surface-400">to</span>
                    <span class="text-xs text-surface-400">$</span>
                    <InputNumber
                      v-model="item.max_cost"
                      placeholder="Max"
                      :min="1"
                      :minFractionDigits="1"
                      :maxFractionDigits="2"
                      class="h-8 rounded-lg w-24"
                      inputClass="py-1 font-mono text-sm text-center"
                    />
                  </div>
                </td>

                <!-- Action Delete -->
                <td class="p-3 text-center">
                  <Button
                    icon="pi pi-trash"
                    severity="danger"
                    text
                    rounded
                    @click="removeSymbol(index)"
                    class="p-button-sm text-rose-500!"
                  />
                </td>
              </tr>

              <!-- Add new Symbol row -->
              <tr class="bg-surface-50/50 dark:bg-surface-950/10">
                <td colspan="5" class="p-3">
                  <div v-if="!isAddingSymbol">
                    <Button
                      icon="pi pi-plus"
                      label="Add Symbol"
                      size="small"
                      severity="secondary"
                      @click="isAddingSymbol = true"
                      class="h-8 rounded-lg text-xs cursor-pointer"
                    />
                  </div>
                  <div v-else class="flex items-center gap-2 max-w-80">
                    <InputText
                      v-model="newSymbolName"
                      placeholder="e.g. SOLUSDT"
                      class="h-8 rounded-lg flex-1 font-mono text-sm"
                      @keyup.enter="confirmAddSymbol"
                      v-focus
                    />
                    <Button
                      icon="pi pi-check"
                      severity="secondary"
                      text
                      size="small"
                      @click="confirmAddSymbol"
                      class="h-8 w-8 rounded-lg cursor-pointer flex items-center justify-center"
                    />
                    <Button
                      icon="pi pi-times"
                      severity="secondary"
                      text
                      size="small"
                      @click="cancelAddSymbol"
                      class="h-8 w-8 rounded-lg cursor-pointer flex items-center justify-center"
                    />
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- Take-Profit / Stop-Loss Rules -->
      <div class="flex flex-col gap-3 mb-6">
        <label class="text-xs font-bold text-surface-500">Take-Profit / Stop-Loss Rules</label>

        <div class="border border-surface-200 dark:border-surface-800 rounded-2xl p-4">
          <div class="flex flex-col gap-4">
            <!-- Take-profit settings -->
            <div class="flex flex-col gap-3 border-b border-surface-200 dark:border-surface-800 pb-4">
              <div class="flex items-center gap-3">
                <label class="text-xs text-surface-500 w-24 shrink-0">Take-Profit</label>
                <Select
                  v-model="takeProfitMode"
                  :options="[
                    { label: 'Fixed Unrealized PnL', value: 'fixed' },
                    { label: 'Custom AI Prompt', value: 'custom' }
                  ]"
                  optionLabel="label"
                  optionValue="value"
                  class="h-10 rounded-xl flex-1"
                />
              </div>
              <div v-if="takeProfitMode === 'fixed'" class="flex flex-col gap-1.5">
                <label class="text-xs font-bold text-surface-500">Take-Profit (PnL Rate)</label>
                <div class="flex items-center gap-1.5">
                  <span class="text-sm text-emerald-500 font-bold">+</span>
                  <InputNumber
                    v-model="takeProfitPnlRate"
                    :min="0"
                    :minFractionDigits="0"
                    :maxFractionDigits="2"
                    suffix="%"
                    placeholder=""
                    class="h-10 rounded-xl"
                    inputClass="font-mono"
                  />
                </div>
                <span class="text-xs text-surface-400">Exchange closes the position when unrealized PnL / margin reaches this rate</span>
              </div>
              <div v-else class="flex flex-col gap-1.5">
                <label class="text-xs font-bold text-surface-500">Take-Profit Instructions</label>
                <Textarea
                  v-model="takeProfitPrompt"
                  rows="3"
                  placeholder="Describe when the AI should take profit..."
                  class="rounded-xl"
                  :autoResize="true"
                />
              </div>
            </div>

            <!-- Stop-loss settings -->
            <div class="flex flex-col gap-3">
              <div class="flex items-center gap-3">
                <label class="text-xs text-surface-500 w-24 shrink-0">Stop-Loss</label>
                <Select
                  v-model="stopLossMode"
                  :options="[
                    { label: 'Fixed Unrealized PnL', value: 'fixed' },
                    { label: 'Custom AI Prompt', value: 'custom' }
                  ]"
                  optionLabel="label"
                  optionValue="value"
                  class="h-10 rounded-xl flex-1"
                />
              </div>
              <div v-if="stopLossMode === 'fixed'" class="flex flex-col gap-1.5">
                <label class="text-xs font-bold text-surface-500">Stop-Loss (PnL Rate)</label>
                <div class="flex items-center gap-1.5">
                  <span class="text-sm text-rose-500 font-bold">-</span>
                  <InputNumber
                    v-model="stopLossPnlRate"
                    :min="0"
                    :minFractionDigits="0"
                    :maxFractionDigits="2"
                    suffix="%"
                    placeholder=""
                    class="h-10 rounded-xl"
                    inputClass="font-mono"
                  />
                </div>
                <span class="text-xs text-surface-400">Exchange closes the position when unrealized PnL / margin drops to this rate</span>
              </div>
              <div v-else class="flex flex-col gap-1.5">
                <label class="text-xs font-bold text-surface-500">Stop-Loss Instructions</label>
                <Textarea
                  v-model="stopLossPrompt"
                  rows="3"
                  placeholder="Describe when the AI should stop the position..."
                  class="rounded-xl"
                  :autoResize="true"
                />
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Action Footer -->
      <div class="flex gap-3 mt-6 border-t border-surface-200 dark:border-surface-800 pt-4">
        <Button
          v-if="!dataTemplateError"
          icon="pi pi-save"
          label="Save"
          @click="emit('save')"
          :loading="saving"
          class="rounded-xl h-11 cursor-pointer flex-1"
        />
        <Button
          v-if="!dataTemplateError"
          icon="pi pi-sparkles"
          label="Test Run (AI)"
          severity="help"
          @click="emit('test')"
          :loading="testRunLoading"
          class="rounded-xl h-11 cursor-pointer"
        />
        <Button
          icon="pi pi-times"
          label="Cancel"
          severity="secondary"
          @click="emit('cancel')"
          class="rounded-xl h-11 cursor-pointer w-32"
        />
      </div>
    </template>
  </Card>
</template>

<style>
.indicator-picker-select {
  position: absolute !important;
  inset: 0 !important;
  width: 100% !important;
  height: 100% !important;
  opacity: 0 !important;
  pointer-events: none !important;
}

.indicator-picker-option {
  display: block;
  width: 100%;
}
</style>
