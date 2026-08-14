<script setup lang="ts">
import InputNumber from "primevue/inputnumber"
import Select from "primevue/select"
import Textarea from "primevue/textarea"
import { computed } from "vue"
import type {
  StrategyConfigPayload,
  StrategyTpSlMode,
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

// tp_sl is guaranteed to exist — ensureStrategyConfig runs in the composable.
// Computed get/set pairs read from and write to config directly.
// No local refs, no initialization flag, no sync watches.

const takeProfitMode = computed<StrategyTpSlMode>({
  get: () => config.value.tp_sl?.take_profit.mode ?? "fixed",
  set: (val) => {
    if (config.value.tp_sl) config.value.tp_sl.take_profit.mode = val
  },
})

const stopLossMode = computed<StrategyTpSlMode>({
  get: () => config.value.tp_sl?.stop_loss.mode ?? "fixed",
  set: (val) => {
    if (config.value.tp_sl) config.value.tp_sl.stop_loss.mode = val
  },
})

// TP rate: stored as ratio (0.10 = +10%), displayed as positive percentage
const takeProfitPnlRate = computed<number | null>({
  get: () => {
    const rate = config.value.tp_sl?.take_profit.pnl_rate
    return rate != null ? Math.abs(rate * 100) : null
  },
  set: (val) => {
    if (config.value.tp_sl) {
      config.value.tp_sl.take_profit.pnl_rate = val != null ? val / 100 : null
    }
  },
})

// SL rate: stored as negative ratio (-0.05 = -5%), displayed as positive percentage
const stopLossPnlRate = computed<number | null>({
  get: () => {
    const rate = config.value.tp_sl?.stop_loss.pnl_rate
    return rate != null ? Math.abs(rate * 100) : null
  },
  set: (val) => {
    if (config.value.tp_sl) {
      config.value.tp_sl.stop_loss.pnl_rate =
        val != null ? -(val / 100) : null
    }
  },
})

const takeProfitPrompt = computed<string>({
  get: () => config.value.tp_sl?.take_profit.custom_prompt ?? "",
  set: (val) => {
    if (config.value.tp_sl) config.value.tp_sl.take_profit.custom_prompt = val
  },
})

const stopLossPrompt = computed<string>({
  get: () => config.value.tp_sl?.stop_loss.custom_prompt ?? "",
  set: (val) => {
    if (config.value.tp_sl) config.value.tp_sl.stop_loss.custom_prompt = val
  },
})
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
