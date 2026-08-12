<script setup lang="ts">
import InputNumber from "primevue/inputnumber"
import Select from "primevue/select"
import Textarea from "primevue/textarea"
import { ref, computed, watch } from "vue"
import type {
  StrategyConfigPayload,
  StrategyTpSlConfigPayload,
  StrategyTpSlMode,
  StrategyTakeProfitConfigPayload,
  StrategyStopLossConfigPayload,
} from "@/types/strategies"
import EditorSection from "./EditorSection.vue"

const props = defineProps<{
  config: StrategyConfigPayload
}>()

const config = computed(() => props.config)

const modeOptions = [
  { label: "Fixed Unrealized PnL", value: "fixed" },
  { label: "Custom AI Prompt", value: "custom" },
]

const takeProfitMode = ref<StrategyTpSlMode>("fixed")
const stopLossMode = ref<StrategyTpSlMode>("fixed")
// Both values are positive percentages in the UI; stop-loss is stored as a negative ratio.
const takeProfitPnlRate = ref<number | null>(null)
const stopLossPnlRate = ref<number | null>(null)
const takeProfitPrompt = ref("")
const stopLossPrompt = ref("")
let tpSlInitialized = false

function emptyTakeProfitRule(): StrategyTakeProfitConfigPayload {
  return {
    mode: "fixed",
    pnl_rate: null,
    custom_prompt: null,
  }
}

function emptyStopLossRule(): StrategyStopLossConfigPayload {
  return {
    mode: "fixed",
    pnl_rate: null,
    custom_prompt: null,
  }
}

function emptyTpSlConfig(): StrategyTpSlConfigPayload {
  return {
    take_profit: emptyTakeProfitRule(),
    stop_loss: emptyStopLossRule(),
  }
}

// Initialize once from existing config (waits for async data if needed)
watch(
  () => config.value.tp_sl,
  (tpSl) => {
    if (tpSlInitialized) return
    if (!tpSl?.take_profit || !tpSl.stop_loss) {
      config.value.tp_sl = emptyTpSlConfig()
      tpSl = config.value.tp_sl
    }
    takeProfitMode.value = tpSl.take_profit.mode
    stopLossMode.value = tpSl.stop_loss.mode
    takeProfitPnlRate.value =
      tpSl.take_profit.pnl_rate != null
        ? Math.abs(tpSl.take_profit.pnl_rate * 100)
        : null
    stopLossPnlRate.value =
      tpSl.stop_loss.pnl_rate != null
        ? Math.abs(tpSl.stop_loss.pnl_rate * 100)
        : null
    takeProfitPrompt.value = tpSl.take_profit.custom_prompt ?? ""
    stopLossPrompt.value = tpSl.stop_loss.custom_prompt ?? ""
    tpSlInitialized = true
  },
  { immediate: true },
)

// Sync each rule back to config — only after initialization.
watch(
  [
    takeProfitMode,
    stopLossMode,
    takeProfitPnlRate,
    stopLossPnlRate,
    takeProfitPrompt,
    stopLossPrompt,
  ],
  () => {
    if (!tpSlInitialized) return
    let tpSlConfig = config.value.tp_sl
    if (!tpSlConfig?.take_profit || !tpSlConfig.stop_loss) {
      tpSlConfig = emptyTpSlConfig()
      config.value.tp_sl = tpSlConfig
    }
    tpSlConfig.take_profit = {
      mode: takeProfitMode.value,
      pnl_rate:
        takeProfitMode.value === "fixed" && takeProfitPnlRate.value != null
          ? takeProfitPnlRate.value / 100
          : null,
      custom_prompt:
        takeProfitMode.value === "custom" ? takeProfitPrompt.value : null,
    }
    tpSlConfig.stop_loss = {
      mode: stopLossMode.value,
      pnl_rate:
        stopLossMode.value === "fixed" && stopLossPnlRate.value != null
          ? -(stopLossPnlRate.value / 100)
          : null,
      custom_prompt:
        stopLossMode.value === "custom" ? stopLossPrompt.value : null,
    }
  },
)
</script>

<template>
  <EditorSection
    icon="pi-bullseye"
    title="Take-Profit / Stop-Loss"
    description="When to close a position — a fixed PnL threshold or custom AI instructions"
  >
    <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
      <!-- Take-profit panel -->
      <div
        class="flex flex-col gap-4 rounded-xl border border-surface-200 dark:border-surface-800 bg-surface-50/50 dark:bg-surface-950/40 p-4"
      >
        <div class="flex flex-wrap items-center gap-3">
          <span
            class="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-emerald-500/10 text-emerald-500"
          >
            <span class="pi pi-arrow-up-right text-sm"></span>
          </span>
          <span
            class="text-xs font-bold uppercase tracking-wider text-emerald-600 dark:text-emerald-400"
            >Take-Profit</span
          >
          <Select
            v-model="takeProfitMode"
            :options="modeOptions"
            optionLabel="label"
            optionValue="value"
            class="ml-auto h-9 rounded-lg text-xs flex items-center"
          />
        </div>
        <div v-if="takeProfitMode === 'fixed'" class="flex flex-col gap-1.5">
          <label
            class="text-xs font-bold text-surface-500 dark:text-surface-400"
            >Take-Profit (PnL Rate)</label
          >
          <div class="flex items-center gap-1.5">
            <span class="text-sm text-emerald-500 font-bold">+</span>
            <InputNumber
              v-model="takeProfitPnlRate"
              :min="0"
              :minFractionDigits="0"
              :maxFractionDigits="2"
              suffix="%"
              class="h-10 rounded-xl flex-1"
              inputClass="font-mono"
            />
          </div>
          <span class="text-xs text-surface-400">
            Exchange closes the position when unrealized PnL / margin reaches
            this rate
          </span>
        </div>
        <div v-else class="flex flex-col gap-1.5">
          <label
            class="text-xs font-bold text-surface-500 dark:text-surface-400"
            >Take-Profit Instructions</label
          >
          <Textarea
            v-model="takeProfitPrompt"
            rows="3"
            placeholder="Describe when the AI should take profit..."
            class="rounded-xl"
            :autoResize="true"
          />
        </div>
      </div>

      <!-- Stop-loss panel -->
      <div
        class="flex flex-col gap-4 rounded-xl border border-surface-200 dark:border-surface-800 bg-surface-50/50 dark:bg-surface-950/40 p-4"
      >
        <div class="flex flex-wrap items-center gap-3">
          <span
            class="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-rose-500/10 text-rose-500"
          >
            <span class="pi pi-arrow-down-right text-sm"></span>
          </span>
          <span
            class="text-xs font-bold uppercase tracking-wider text-rose-600 dark:text-rose-400"
            >Stop-Loss</span
          >
          <Select
            v-model="stopLossMode"
            :options="modeOptions"
            optionLabel="label"
            optionValue="value"
            class="ml-auto h-9 rounded-lg text-xs flex items-center"
          />
        </div>
        <div v-if="stopLossMode === 'fixed'" class="flex flex-col gap-1.5">
          <label
            class="text-xs font-bold text-surface-500 dark:text-surface-400"
            >Stop-Loss (PnL Rate)</label
          >
          <div class="flex items-center gap-1.5">
            <span class="text-sm text-rose-500 font-bold">-</span>
            <InputNumber
              v-model="stopLossPnlRate"
              :min="0"
              :minFractionDigits="0"
              :maxFractionDigits="2"
              suffix="%"
              class="h-10 rounded-xl flex-1"
              inputClass="font-mono"
            />
          </div>
          <span class="text-xs text-surface-400">
            Exchange closes the position when unrealized PnL / margin drops to
            this rate
          </span>
        </div>
        <div v-else class="flex flex-col gap-1.5">
          <label
            class="text-xs font-bold text-surface-500 dark:text-surface-400"
            >Stop-Loss Instructions</label
          >
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
  </EditorSection>
</template>
