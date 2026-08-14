<script setup lang="ts">
import type { EditableStrategy } from "@/types/strategy-ui"
import { formatDate } from "@/utils/format"

defineProps<{
  strategies: EditableStrategy[]
  loading: boolean
}>()

const emit = defineEmits<{
  select: [strategy: EditableStrategy]
}>()
</script>

<template>
  <div class="flex flex-col gap-4 w-full">
    <div
      v-for="strategy in strategies"
      :key="strategy.id"
      @click="emit('select', strategy)"
      class="w-full transition-all duration-200"
    >
      <!-- Custom Card with Fixed Height (h-32) and Full Width -->
      <div
        class="w-full h-32 p-4 border rounded-2xl transition-all duration-200 select-none cursor-pointer flex items-center hover:shadow-sm border-surface-200 dark:border-surface-800 bg-surface-0 dark:bg-surface-900 hover:border-surface-300 dark:hover:border-surface-700 hover:bg-surface-50/30 dark:hover:bg-surface-950/20"
      >
        <div class="flex items-center justify-between h-full w-full gap-6">
          <!-- Column 1: Name, Description, and Active Status -->
          <div
            class="flex-1 min-w-50 flex flex-col justify-between h-full py-0.5"
          >
            <div>
              <div class="flex items-center gap-2 mb-1">
                <span
                  class="font-bold text-sm text-surface-900 dark:text-white truncate max-w-55"
                >
                  {{ strategy.name }}
                </span>
              </div>
              <p
                class="text-xs line-clamp-2 text-surface-500 dark:text-surface-400 pr-2"
              >
                {{ strategy.description || "No description provided." }}
              </p>
            </div>
            <p class="text-xs text-surface-400 dark:text-surface-500 font-mono">
              Updated {{ formatDate(strategy.updated_at) }}
            </p>
          </div>

          <!-- Column 2: Parameters (Max Positions, Prompt Variant) -->
          <div
            class="w-45 border-l border-surface-200 dark:border-surface-800 pl-6 flex flex-col justify-center gap-2 h-full"
          >
            <div class="flex items-center justify-between text-xs">
              <span class="text-surface-400 dark:text-surface-500"
                >Max Positions:</span
              >
              <span
                class="font-bold font-mono text-surface-900 dark:text-white"
              >
                {{ strategy.config?.max_positions ?? 5 }}
              </span>
            </div>
            <div class="flex items-center justify-between text-xs">
              <span class="text-surface-400 dark:text-surface-500"
                >Prompt Variant:</span
              >
              <span
                class="font-bold capitalize text-surface-900 dark:text-white text-xs"
              >
                {{ strategy.config?.prompt_variant ?? "balanced" }}
              </span>
            </div>
          </div>

          <!-- Column 3: Trading Targets (Symbols, Leverage, Cost settings) -->
          <div
            class="flex-1 min-w-75 border-l border-surface-200 dark:border-surface-800 pl-6 flex flex-col justify-center gap-2 h-full"
          >
            <span
              class="text-xs font-bold text-surface-400 dark:text-surface-500 uppercase tracking-wider"
              >Trading Targets</span
            >

            <div class="flex flex-wrap gap-1.5 max-h-16 overflow-y-auto pr-1">
              <span
                v-if="
                  !strategy.config?.symbols ||
                  strategy.config.symbols.length === 0
                "
                class="text-xs text-surface-400 dark:text-surface-500 italic py-1"
              >
                No symbols configured
              </span>
              <div
                v-else
                v-for="sym in strategy.config?.symbols"
                :key="sym.symbol"
                class="flex items-center gap-1 bg-surface-50 dark:bg-surface-950 border border-surface-200 dark:border-surface-800 px-2 py-0.5 rounded-md text-xs font-mono"
              >
                <span class="font-bold text-surface-900 dark:text-white">{{
                  sym.symbol
                }}</span>
                <span class="text-surface-400 dark:text-surface-500"
                  >({{ sym.leverage }}x)</span
                >
                <span class="text-surface-500 dark:text-surface-400 ml-1">
                  {{
                    sym.fixed_cost != null
                      ? `$${sym.fixed_cost}`
                      : `$${sym.min_cost}-$${sym.max_cost}`
                  }}
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Empty State -->
    <div
      v-if="!loading && strategies.length === 0"
      class="w-full text-center py-12 text-sm text-surface-400 dark:text-surface-500 border border-dashed border-surface-200 dark:border-surface-800 rounded-2xl bg-surface-50/50 dark:bg-surface-950/20"
    >
      No strategies created yet.
    </div>

    <!-- Loading State -->
    <div
      v-if="loading"
      class="w-full text-center py-12 text-sm text-surface-400 dark:text-surface-500"
    >
      <span class="pi pi-spin pi-spinner mr-2"></span>
      Loading strategies...
    </div>
  </div>
</template>
