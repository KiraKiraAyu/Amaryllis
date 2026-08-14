<script setup lang="ts">
import Button from "primevue/button"
import InputNumber from "primevue/inputnumber"
import Select from "primevue/select"
import MultiSelect from "primevue/multiselect"
import { ref, computed, watch, nextTick } from "vue"
import type {
  StrategyConfigPayload,
  StrategyDataItemPayload,
  StrategyIndicatorType,
} from "@/types/strategies"
import EditorSection from "./EditorSection.vue"
import {
  timeframeOptions,
  indicatorOptions,
  getDataTemplate,
  dataItemLabel,
  computeDataTemplateError,
  computeItemError,
} from "./data-template"

const props = defineProps<{
  config: StrategyConfigPayload
  strategyId: string
}>()

const config = computed(() => props.config)

// Pure read — data_template is guaranteed by ensureStrategyConfig in the composable.
// getDataTemplate returns a fallback default if missing, but does NOT mutate config.
const dataTemplate = computed(() => getDataTemplate(config.value))
const dataItems = computed(() => dataTemplate.value.items)
const rawKlineItems = computed(() =>
  dataItems.value.filter((item) => item.type === "raw_kline"),
)
const manuallyEditedIndicatorTimeframes = ref(new Set<string>())
const indicatorPickerValue = ref<StrategyIndicatorType | null>(null)
const indicatorPicker = ref<{ show: () => void } | null>(null)

const dataTemplateError = computed(() =>
  computeDataTemplateError(dataItems.value),
)

function addKlineItem() {
  if (rawKlineItems.value.length >= 1) return
  const id = `kline-${Date.now()}`
  dataItems.value.push({
    id,
    type: "raw_kline",
    timeframes: ["5m"],
    count: 100,
    closed_only: true,
    output_mode: "latest",
  })
}

function addIndicatorItem(indicator: StrategyIndicatorType) {
  const id = `indicator-${Date.now()}`
  const kline = rawKlineItems.value[0]
  const timeframes: StrategyDataItemPayload["timeframes"] = kline
    ? [...kline.timeframes]
    : ["5m"]
  dataItems.value.push({
    id,
    type: "indicator",
    timeframes,
    indicator,
    params: { period: 14 },
    output_mode: "latest",
  })
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

function setIndicatorPeriod(
  item: StrategyDataItemPayload,
  value: number | null,
) {
  item.params = { ...item.params, period: value ?? 14 }
}

function setIndicatorTimeframes(
  item: StrategyDataItemPayload,
  value: StrategyDataItemPayload["timeframes"] | null,
) {
  manuallyEditedIndicatorTimeframes.value.add(item.id)
  item.timeframes = value ?? []
}

function followsKlineTimeframes(item: StrategyDataItemPayload) {
  return (
    item.type === "indicator" &&
    !manuallyEditedIndicatorTimeframes.value.has(item.id) &&
    rawKlineItems.value.length > 0
  )
}

watch(
  () => rawKlineItems.value[0]?.timeframes,
  (timeframes) => {
    if (!timeframes) return
    for (const item of dataItems.value) {
      if (
        item.type === "indicator" &&
        !manuallyEditedIndicatorTimeframes.value.has(item.id)
      ) {
        item.timeframes = [...timeframes]
      }
    }
  },
  { deep: true },
)

watch(
  () => props.strategyId,
  () => {
    manuallyEditedIndicatorTimeframes.value.clear()
  },
)
</script>

<template>
  <EditorSection
    icon="pi-database"
    title="Market Data"
    description="Klines and indicators fed into the AI prompt on every scan"
  >
    <div class="flex flex-col gap-3">
      <div class="flex flex-wrap items-center gap-2">
        <Button
          icon="pi pi-plus"
          label="Add Kline"
          size="small"
          severity="secondary"
          :disabled="rawKlineItems.length >= 1"
          @click="addKlineItem"
        />
        <div class="relative">
          <Button
            icon="pi pi-plus"
            label="Add Indicator"
            size="small"
            severity="secondary"
            @click="openIndicatorPicker"
          />
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
              <span
                class="p-select-option-label indicator-picker-option"
                @mousedown.stop.prevent="selectIndicator(option.value)"
              >
                {{ option.label }}
              </span>
            </template>
          </Select>
        </div>
      </div>

      <div v-if="dataItems.length === 0" class="text-sm text-rose-500">
        {{ dataTemplateError }}
      </div>

      <div
        v-for="(item, index) in dataItems"
        :key="item.id || index"
        class="flex flex-col gap-3 rounded-xl border p-3.5"
        :class="computeItemError(item) ? 'border-rose-400 dark:border-rose-500' : 'border-surface-200 dark:border-surface-800'"
      >
        <div class="flex items-center gap-2.5">
          <span
            class="flex h-7 w-7 shrink-0 items-center justify-center rounded-lg bg-primary-500/10 dark:bg-primary-500/20 text-primary"
          >
            <span
              :class="[
                'pi',
                'text-xs',
                item.type === 'raw_kline' ? 'pi-chart-line' : 'pi-chart-bar',
              ]"
            ></span>
          </span>
          <span
            class="truncate text-sm font-bold text-surface-900 dark:text-white"
          >
            {{ dataItemLabel(item) }}
          </span>
          <span
            v-if="followsKlineTimeframes(item)"
            class="ml-auto hidden sm:flex items-center gap-1 text-xs text-surface-400"
          >
            <span class="pi pi-link text-[0.65rem]"></span>
            Follows Kline timeframes
          </span>
          <Button
            icon="pi pi-trash"
            severity="danger"
            text
            rounded
            aria-label="Remove data item"
            :class="[
              'p-button-sm text-rose-500!',
              followsKlineTimeframes(item) ? '' : 'ml-auto',
            ]"
            @click="removeDataItem(index)"
          />
        </div>

        <div
          class="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:flex lg:flex-wrap lg:items-end"
        >
          <div class="flex min-w-0 flex-1 flex-col gap-1.5">
            <label
              class="text-xs font-bold text-surface-500 dark:text-surface-400"
              >Timeframes</label
            >
            <MultiSelect
              v-if="item.type === 'raw_kline'"
              v-model="item.timeframes"
              :options="timeframeOptions"
              display="chip"
              :maxSelectedLabels="3"
              class="min-h-9 min-w-0 rounded-lg"
            />
            <MultiSelect
              v-else
              :modelValue="item.timeframes"
              @update:modelValue="setIndicatorTimeframes(item, $event)"
              :options="timeframeOptions"
              display="chip"
              :maxSelectedLabels="2"
              class="min-h-9 min-w-0 rounded-lg"
            />
          </div>

          <template v-if="item.type === 'raw_kline'">
            <div class="flex w-full flex-col gap-1.5 sm:w-auto">
              <label
                class="text-xs font-bold text-surface-500 dark:text-surface-400"
                >Kline Count</label
              >
              <InputNumber
                v-model="item.count"
                :min="1"
                :max="1500"
                showButtons
                class="h-9 min-w-24 rounded-lg"
              />
            </div>
            <label
              class="flex items-center gap-2 whitespace-nowrap pb-2 text-xs font-semibold text-surface-500 dark:text-surface-400"
            >
              <input
                v-model="item.closed_only"
                type="checkbox"
                class="accent-primary"
              />
              Closed candles only
            </label>
          </template>

          <div v-else class="flex w-full flex-col gap-1.5 sm:w-auto">
            <label
              class="text-xs font-bold text-surface-500 dark:text-surface-400"
              >Period</label
            >
            <InputNumber
              :modelValue="indicatorPeriod(item)"
              @update:modelValue="setIndicatorPeriod(item, $event)"
              :min="1"
              :max="1500"
              showButtons
              class="h-9 min-w-24 rounded-lg"
            />
          </div>
        </div>

        <p
          v-if="computeItemError(item)"
          class="flex items-center gap-1.5 text-xs font-semibold text-rose-500"
        >
          <span class="pi pi-exclamation-circle shrink-0"></span>
          {{ computeItemError(item) }}
        </p>
      </div>
    </div>
  </EditorSection>
</template>

<style>
.indicator-picker-select {
  position: absolute !important;
  inset: 0 !important;
  width: 100% !important;
  height: 100% !important;
  opacity: 0 !important;
}

.indicator-picker-option {
  display: block;
  width: 100%;
}
</style>
