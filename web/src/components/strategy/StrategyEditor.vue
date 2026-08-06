<script setup lang="ts">
import Card from "primevue/card"
import Button from "primevue/button"
import InputText from "primevue/inputtext"
import InputNumber from "primevue/inputnumber"
import Select from "primevue/select"
import Textarea from "primevue/textarea"
import { ref, computed, watch } from "vue"
import type { EditableStrategy } from "@/types/strategy-ui"

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

const tpSlMode = ref<'fixed' | 'custom'>('fixed')
// Both TP and SL are positive percentages in the UI (e.g. 100 = 100%)
const fixedTpPnlRate = ref<number | null>(null)
const fixedSlPnlRate = ref<number | null>(null)
const customTpSlPrompt = ref<string>('')
let tpSlInitialized = false

// Initialize once from existing config (waits for async data if needed)
watch(() => config.value.tp_sl, (tpSl) => {
  if (tpSlInitialized) return
  if (!tpSl) {
    config.value.tp_sl = {
      mode: 'fixed',
      fixed_tp_pnl_rate: null,
      fixed_sl_pnl_rate: null,
      custom_prompt: null
    }
    tpSlMode.value = 'fixed'
    fixedTpPnlRate.value = null
    fixedSlPnlRate.value = null
    customTpSlPrompt.value = ''
    tpSlInitialized = true
    return
  }
  if (!tpSl.mode) {
    tpSl.mode = 'fixed'
    tpSl.fixed_tp_pnl_rate = null
    tpSl.fixed_sl_pnl_rate = null
    tpSl.custom_prompt = null
    tpSlMode.value = 'fixed'
    fixedTpPnlRate.value = null
    fixedSlPnlRate.value = null
    customTpSlPrompt.value = ''
    tpSlInitialized = true
    return
  }
  if (tpSl.mode) tpSlMode.value = tpSl.mode
  fixedTpPnlRate.value = tpSl.fixed_tp_pnl_rate != null
    ? tpSl.fixed_tp_pnl_rate * 100
    : null
  // SL stored as negative ratio in config, display as positive in UI
  fixedSlPnlRate.value = tpSl.fixed_sl_pnl_rate != null
    ? Math.abs(tpSl.fixed_sl_pnl_rate * 100)
    : null
  if (tpSl.custom_prompt != null) customTpSlPrompt.value = tpSl.custom_prompt
  tpSlInitialized = true
}, { immediate: true })

// Sync back to config (TP positive, SL negative) — only after initialization
watch([tpSlMode, fixedTpPnlRate, fixedSlPnlRate, customTpSlPrompt], () => {
  if (!tpSlInitialized) return
  let tpSlConfig = config.value.tp_sl
  if (!tpSlConfig) {
    tpSlConfig = {
      mode: tpSlMode.value,
      fixed_tp_pnl_rate: null,
      fixed_sl_pnl_rate: null,
      custom_prompt: null
    }
    config.value.tp_sl = tpSlConfig
  }
  tpSlConfig.mode = tpSlMode.value
  tpSlConfig.fixed_tp_pnl_rate = tpSlMode.value === 'fixed' && fixedTpPnlRate.value != null
    ? fixedTpPnlRate.value / 100
    : null
  tpSlConfig.fixed_sl_pnl_rate = tpSlMode.value === 'fixed' && fixedSlPnlRate.value != null
    ? -(fixedSlPnlRate.value / 100)
    : null
  tpSlConfig.custom_prompt = tpSlMode.value === 'custom' ? customTpSlPrompt.value : null
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
            <!-- Mode selector -->
            <div class="flex items-center gap-3">
              <label class="text-xs text-surface-500 w-20 shrink-0">Mode</label>
              <Select
                v-model="tpSlMode"
                :options="[
                  { label: 'Fixed Unrealized PnL', value: 'fixed' },
                  { label: 'Custom AI Prompt', value: 'custom' }
                ]"
                optionLabel="label"
                optionValue="value"
                class="h-10 rounded-xl flex-1"
              />
            </div>

            <!-- Fixed mode: TP/SL PnL rate inputs -->
            <div v-if="tpSlMode === 'fixed'" class="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <div class="flex flex-col gap-1.5">
                <label class="text-xs font-bold text-surface-500">Take-Profit (PnL Rate)</label>
                <div class="flex items-center gap-1.5">
                  <span class="text-sm text-emerald-500 font-bold">+</span>
                  <InputNumber
                    v-model="fixedTpPnlRate"
                    :min="0"
                    :minFractionDigits="0"
                    :maxFractionDigits="2"
                    suffix="%"
                    placeholder=""
                    class="h-10 rounded-xl flex-1"
                    inputClass="font-mono"
                  />
                </div>
                <span class="text-xs text-surface-400">Auto-close when unrealized PnL / margin reaches this rate</span>
              </div>
              <div class="flex flex-col gap-1.5">
                <label class="text-xs font-bold text-surface-500">Stop-Loss (PnL Rate)</label>
                <div class="flex items-center gap-1.5">
                  <span class="text-sm text-rose-500 font-bold">-</span>
                  <InputNumber
                    v-model="fixedSlPnlRate"
                    :min="0"
                    :minFractionDigits="0"
                    :maxFractionDigits="2"
                    suffix="%"
                    placeholder=""
                    class="h-10 rounded-xl flex-1"
                    inputClass="font-mono"
                  />
                </div>
                <span class="text-xs text-surface-400">Auto-close when unrealized PnL / margin drops to this rate</span>
              </div>
            </div>

            <!-- Custom mode: prompt textarea -->
            <div v-if="tpSlMode === 'custom'" class="flex flex-col gap-1.5">
              <label class="text-xs font-bold text-surface-500">Custom TP/SL Instructions</label>
              <Textarea
                v-model="customTpSlPrompt"
                rows="4"
                placeholder="Describe the conditions under which the AI should close positions. e.g. 'Close long positions when RSI exceeds 70 and price hits resistance level...'"
                class="rounded-xl"
                :autoResize="true"
              />
              <span class="text-xs text-surface-400">The AI will use these instructions to decide when to close positions</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Action Footer -->
      <div class="flex gap-3 mt-6 border-t border-surface-200 dark:border-surface-800 pt-4">
        <Button
          icon="pi pi-save"
          label="Save"
          @click="emit('save')"
          :loading="saving"
          class="rounded-xl h-11 cursor-pointer flex-1"
        />
        <Button
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
