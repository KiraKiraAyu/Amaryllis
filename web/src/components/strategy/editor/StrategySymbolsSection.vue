<script setup lang="ts">
import Button from "primevue/button"
import InputText from "primevue/inputtext"
import InputNumber from "primevue/inputnumber"
import Select from "primevue/select"
import { ref, computed } from "vue"
import type { StrategyConfigPayload } from "@/types/strategies"
import EditorSection from "./EditorSection.vue"

const props = defineProps<{
  config: StrategyConfigPayload
}>()

const config = computed(() => props.config)

// symbols is guaranteed to exist — ensureStrategyConfig runs in the composable.
// Pure read with fallback, no mutation in getter.
const symbols = computed({
  get: () => config.value.symbols ?? [],
  set: (val) => {
    config.value.symbols = val
  },
})

const newSymbolName = ref("")
const isAddingSymbol = ref(false)

const vFocus = {
  mounted: (el: HTMLInputElement) => {
    if (el.tagName === "INPUT") {
      el.focus()
    } else {
      el.querySelector("input")?.focus()
    }
  },
}

function confirmAddSymbol() {
  const sym = newSymbolName.value.trim().toUpperCase()
  if (!sym) {
    isAddingSymbol.value = false
    return
  }
  if (symbols.value.some((s) => s.symbol === sym)) {
    return
  }
  symbols.value.push({
    symbol: sym,
    leverage: 5,
    min_cost: 20,
    max_cost: 1000,
    fixed_cost: null,
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

function getCostMode(item: (typeof symbols.value)[number]) {
  return item.fixed_cost != null ? "fixed" : "dynamic"
}

function changeCostMode(index: number, mode: "fixed" | "dynamic") {
  const item = symbols.value[index]
  if (mode === "fixed") {
    item.fixed_cost = 50.0
    item.min_cost = null
    item.max_cost = null
  } else {
    item.fixed_cost = null
    item.min_cost = 20.0
    item.max_cost = 1000.0
  }
}
</script>

<template>
  <EditorSection
    icon="pi-chart-line"
    title="Trading Symbols"
    description="Symbols this strategy trades, with per-symbol leverage and cost limits"
  >
    <template #actions>
      <span
        class="rounded-lg bg-surface-100 dark:bg-surface-800 px-2 py-0.5 text-xs font-semibold font-mono text-surface-500 dark:text-surface-400"
      >
        {{ symbols.length }}
      </span>
    </template>

    <div
      class="overflow-x-auto border border-surface-200 dark:border-surface-800 rounded-2xl"
    >
      <table class="w-full text-left border-collapse min-w-150">
        <thead>
          <tr
            class="bg-surface-50 dark:bg-surface-950 border-b border-surface-200 dark:border-surface-800"
          >
            <th
              class="p-3 text-xs font-bold text-surface-500 uppercase tracking-wider w-30"
            >
              Symbol
            </th>
            <th
              class="p-3 text-xs font-bold text-surface-500 uppercase tracking-wider w-30"
            >
              Leverage
            </th>
            <th
              class="p-3 text-xs font-bold text-surface-500 uppercase tracking-wider w-30"
            >
              Cost Mode
            </th>
            <th
              class="p-3 text-xs font-bold text-surface-500 uppercase tracking-wider"
            >
              Cost Settings
            </th>
            <th
              class="p-3 text-xs font-bold text-surface-500 uppercase tracking-wider w-20 text-center"
            >
              Actions
            </th>
          </tr>
        </thead>
        <tbody class="divide-y divide-surface-200 dark:divide-surface-800">
          <tr v-if="symbols.length === 0">
            <td colspan="5" class="p-4 text-center text-sm text-surface-400">
              No symbols added. Add a symbol below to start trading.
            </td>
          </tr>
          <tr
            v-for="(item, index) in symbols"
            :key="item.symbol"
            class="hover:bg-surface-50/50 dark:hover:bg-surface-950/20"
          >
            <!-- Symbol -->
            <td
              class="p-3 text-sm font-bold text-surface-900 dark:text-white font-mono"
            >
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
                @update:modelValue="
                  changeCostMode(index, $event as 'fixed' | 'dynamic')
                "
                :options="[
                  { label: 'Fixed', value: 'fixed' },
                  { label: 'Dynamic', value: 'dynamic' },
                ]"
                optionLabel="label"
                optionValue="value"
                class="h-8 rounded-lg w-27.5 text-xs flex items-center"
              />
            </td>

            <!-- Cost Settings -->
            <td class="p-3">
              <div
                v-if="getCostMode(item) === 'fixed'"
                class="flex items-center gap-1.5 max-w-37.5"
              >
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
                aria-label="Remove symbol"
                @click="removeSymbol(index)"
                class="p-button-sm text-rose-500!"
              />
            </td>
          </tr>

          <!-- Add new Symbol row -->
          <tr class="bg-surface-50/50 dark:bg-surface-950/10">
            <td colspan="5" class="p-3">
              <button
                v-if="!isAddingSymbol"
                type="button"
                class="flex h-9 w-full cursor-pointer items-center justify-center gap-2 rounded-lg border border-dashed border-surface-300 dark:border-surface-700 text-xs font-semibold text-surface-400 transition-colors hover:border-primary/50 hover:text-primary"
                @click="isAddingSymbol = true"
              >
                <span class="pi pi-plus text-[0.65rem]"></span>
                Add Symbol
              </button>
              <div v-else class="flex items-center gap-2 max-w-80">
                <InputText
                  v-model="newSymbolName"
                  placeholder="e.g. SOLUSDT"
                  class="h-8 rounded-lg flex-1 font-mono text-sm"
                  @keyup.enter="confirmAddSymbol"
                  @keyup.escape="cancelAddSymbol"
                  v-focus
                />
                <Button
                  icon="pi pi-check"
                  severity="secondary"
                  text
                  size="small"
                  aria-label="Confirm add symbol"
                  @click="confirmAddSymbol"
                  class="h-8 w-8 rounded-lg cursor-pointer flex items-center justify-center"
                />
                <Button
                  icon="pi pi-times"
                  severity="secondary"
                  text
                  size="small"
                  aria-label="Cancel add symbol"
                  @click="cancelAddSymbol"
                  class="h-8 w-8 rounded-lg cursor-pointer flex items-center justify-center"
                />
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </EditorSection>
</template>
