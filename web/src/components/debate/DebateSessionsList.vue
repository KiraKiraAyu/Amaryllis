<script setup lang="ts">
import Card from "primevue/card"
import type { DebateSession } from "@/types/debate-ui"
import { formatDateTime } from "@/utils/format"

defineProps<{
  debates: DebateSession[]
  activeId?: string
  loading: boolean
}>()

const emit = defineEmits<{
  select: [debate: DebateSession]
}>()
</script>

<template>
  <div class="flex flex-col gap-3">
    <h2
      class="font-bold text-sm text-surface-400 dark:text-surface-500 uppercase tracking-wider mb-1"
    >
      Sessions
    </h2>
    <div
      v-for="debate in debates"
      :key="debate.id"
      @click="emit('select', debate)"
      class="cursor-pointer transition-all duration-200"
    >
      <Card
        class="border transition-all duration-200 shadow-none! select-none"
        :class="
          activeId === debate.id
            ? 'border-primary bg-primary-50/10 dark:bg-primary-950/20'
            : 'border-surface-200 dark:border-surface-800 bg-surface-0 dark:bg-surface-900 hover:border-surface-300 dark:hover:border-surface-700'
        "
      >
        <template #content>
          <div class="flex items-center justify-between mb-2 gap-2">
            <span
              class="font-bold text-sm text-surface-900 dark:text-white truncate"
            >
              {{ debate.name || debate.symbol }}
            </span>
            <span
              class="text-xs font-black uppercase tracking-wider px-1.5 py-0.5 rounded"
              :class="
                debate.status === 'completed'
                  ? 'bg-emerald-500/10 text-emerald-500'
                  : debate.status === 'running'
                    ? 'bg-amber-500/10 text-amber-500 animate-pulse'
                    : 'bg-surface-100 text-surface-500 dark:bg-surface-800 dark:text-surface-400'
              "
            >
              {{ debate.status }}
            </span>
          </div>
          <p class="text-xs text-surface-500 dark:text-surface-400 font-medium">
            Symbol:
            <span class="font-bold text-primary font-mono">{{
              debate.symbol
            }}</span>
            · {{ debate.max_rounds }} rounds ·
            {{ debate.current_round }} completed
          </p>
          <p class="text-xs mt-3 text-surface-400 dark:text-surface-500">
            Created {{ formatDateTime(debate.created_at) }}
          </p>
        </template>
      </Card>
    </div>
    <div
      v-if="!loading && debates.length === 0"
      class="text-center py-12 text-sm text-surface-400 dark:text-surface-500 border border-dashed border-surface-200 dark:border-surface-800 rounded-2xl bg-surface-50/50 dark:bg-surface-950/20"
    >
      No debates created yet.
    </div>
  </div>
</template>
