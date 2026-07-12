<script setup lang="ts">
import Card from "primevue/card"
import Button from "primevue/button"
import EquityChart from "@/components/EquityChart.vue"

defineProps<{
  traderIds: string[]
  activeTraderId: string
  data: { time: number; value: number }[]
  traderName: (id: string) => string
}>()

const emit = defineEmits<{
  select: [traderId: string]
}>()
</script>

<template>
  <Card class="border border-surface-200 dark:border-surface-800 bg-surface-0 dark:bg-surface-900 shadow-none!">
    <template #content>
      <div class="flex items-center justify-between mb-4 flex-wrap gap-2">
        <h2 class="font-bold text-lg text-surface-900 dark:text-white">Equity Curve</h2>
        <div class="flex gap-1 bg-surface-100/50 dark:bg-surface-950/50 p-1 rounded-xl border border-surface-200 dark:border-surface-800">
          <Button
            v-for="traderId in traderIds"
            :key="traderId"
            @click="emit('select', traderId)"
            :label="traderName(traderId)"
            text
            size="small"
            class="px-3 py-1.5 rounded-lg text-xs font-semibold! transition-all duration-250 cursor-pointer"
            :class="
              activeTraderId === traderId
                ? 'bg-primary! text-primary-contrast! shadow-xs'
                : 'text-surface-500 hover:text-primary dark:hover:text-primary-400'
            "
          />
        </div>
      </div>
      <EquityChart :data="data" :height="220" color="oklch(0.66 0.058 301)" />
    </template>
  </Card>
</template>
