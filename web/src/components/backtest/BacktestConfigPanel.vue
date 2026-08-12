<script setup lang="ts">
import Card from "primevue/card"
import Select from "primevue/select"
import InputText from "primevue/inputtext"
import InputNumber from "primevue/inputnumber"
import type { BacktestConfig } from "@/types/backtest-ui"

const cfg = defineModel<BacktestConfig>({ required: true })
const intervals = ["1m", "5m", "15m", "1h", "4h", "1d"]
</script>

<template>
  <Card
    class="border border-surface-200 dark:border-surface-800 bg-surface-0 dark:bg-surface-900 shadow-none!"
  >
    <template #content>
      <h2 class="font-bold text-lg text-surface-900 dark:text-white mb-5">
        Backtest Configuration
      </h2>
      <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <div class="flex flex-col gap-1.5">
          <label class="text-xs font-bold text-surface-500"
            >Time Interval</label
          >
          <Select
            v-model="cfg.interval"
            :options="intervals"
            placeholder="Select interval"
            class="h-10 rounded-xl flex items-center"
          />
        </div>
        <div class="flex flex-col gap-1.5">
          <label class="text-xs font-bold text-surface-500">Start Date</label>
          <InputText
            v-model="cfg.startDate"
            type="date"
            class="h-10 rounded-xl font-mono"
          />
        </div>
        <div class="flex flex-col gap-1.5">
          <label class="text-xs font-bold text-surface-500">End Date</label>
          <InputText
            v-model="cfg.endDate"
            type="date"
            class="h-10 rounded-xl font-mono"
          />
        </div>
        <div class="flex flex-col gap-1.5">
          <label class="text-xs font-bold text-surface-500"
            >Initial Balance (USD)</label
          >
          <InputNumber
            v-model="cfg.initial_balance"
            :min="100"
            mode="currency"
            currency="USD"
            locale="en-US"
            class="h-10 rounded-xl"
            inputClass="font-mono"
          />
        </div>
      </div>
    </template>
  </Card>
</template>
