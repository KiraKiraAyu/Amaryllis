<script setup lang="ts">
import { computed } from "vue"
import Card from "primevue/card"

const props = defineProps<{
  title: string
  value: number
  prefix?: string
  signed?: boolean
  loading?: boolean
}>()

const formatted = computed(() => {
  const v = props.value ?? 0
  const opts: Intl.NumberFormatOptions = {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }
  if (typeof v === "number") {
    // If it's Active Traders (which has no prefix and is usually integer), format differently
    if (!props.prefix) {
      return v.toLocaleString("en-US", { maximumFractionDigits: 0 })
    }
    return (props.signed && v >= 0 ? "+" : "") + v.toLocaleString("en-US", opts)
  }
  return String(v)
})

// Dynamic visual config based on the title
const config = computed(() => {
  const lower = props.title.toLowerCase()
  if (lower.includes("equity")) {
    return {
      icon: "pi pi-wallet",
      tone: "bg-blue-50 text-blue-600 dark:bg-blue-950/30 dark:text-blue-400",
      accent: "from-blue-500 to-cyan-400",
    }
  } else if (lower.includes("cash")) {
    return {
      icon: "pi pi-money-bill",
      tone: "bg-emerald-50 text-emerald-600 dark:bg-emerald-950/30 dark:text-emerald-400",
      accent: "from-emerald-500 to-teal-400",
    }
  } else if (lower.includes("pnl")) {
    const isPositive = (props.value ?? 0) >= 0
    return {
      icon: "pi pi-chart-line",
      tone: isPositive
        ? "bg-emerald-50 text-emerald-600 dark:bg-emerald-950/30 dark:text-emerald-400"
        : "bg-rose-50 text-rose-600 dark:bg-rose-950/30 dark:text-rose-400",
      accent: isPositive
        ? "from-emerald-500 to-teal-400"
        : "from-rose-500 to-red-400",
    }
  } else {
    return {
      icon: "pi pi-users",
      tone: "bg-purple-50 text-purple-600 dark:bg-purple-950/30 dark:text-purple-400",
      accent: "from-purple-500 to-reisa-lilac-400",
    }
  }
})
</script>

<template>
  <Card
    class="overflow-hidden border border-surface-200 dark:border-surface-800 bg-surface-0 dark:bg-surface-900 shadow-none!"
  >
    <template #content>
      <div class="flex items-start justify-between gap-3">
        <div class="flex-1 min-w-0">
          <div
            class="text-xs font-bold uppercase tracking-wider text-surface-400 dark:text-surface-500 truncate"
          >
            {{ title }}
          </div>
          <div
            v-if="loading"
            class="mt-3 h-8 w-2/3 rounded-lg animate-pulse bg-surface-100 dark:bg-surface-800"
          ></div>
          <div
            v-else
            class="mt-2 text-2xl font-black font-display text-surface-900 dark:text-white truncate"
          >
            <span
              v-if="prefix"
              class="text-sm font-semibold text-surface-400 dark:text-surface-500 mr-0.5"
            >
              {{ prefix }}
            </span>
            {{ formatted }}
          </div>
        </div>
        <div
          :class="[
            'grid h-10 w-10 place-items-center rounded-xl shrink-0 transition-transform duration-300 hover:scale-105',
            config.tone,
          ]"
        >
          <span :class="config.icon" class="text-base"></span>
        </div>
      </div>
      <!-- Decorative Progress bar -->
      <div
        class="mt-4 h-1 overflow-hidden rounded-full bg-surface-100 dark:bg-surface-800"
      >
        <div
          class="h-full rounded-full bg-linear-to-r transition-all duration-1000"
          :class="config.accent"
          :style="{ width: loading ? '10%' : '100%' }"
        ></div>
      </div>
    </template>
  </Card>
</template>
