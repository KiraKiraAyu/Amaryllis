<script setup lang="ts">
import type { RuntimeEventPayload } from "@/types/trading"
import { formatTime } from "@/utils/format"

defineProps<{
  events: RuntimeEventPayload[]
  loading: boolean
  formatMetadata: (value: unknown) => string
}>()
</script>

<template>
  <div class="flex flex-col h-175">
    <div class="flex items-center justify-between mb-4">
      <h2 class="font-bold text-sm text-surface-900 dark:text-white">Runtime Events</h2>
      <span class="text-surface-500 text-sm">{{ events.length }} events</span>
    </div>
    <div class="flex-1 overflow-y-auto pr-2 flex flex-col gap-2">
      <div
        v-if="loading"
        class="text-center py-10 text-xs text-surface-500"
      >
        Loading events...
      </div>
      <div
        v-else-if="events.length === 0"
        class="text-center py-10 text-xs text-surface-500"
      >
        No events.
      </div>
      <div
        v-for="event in events"
        v-else
        :key="event.id"
        class="p-3 rounded-xl border flex flex-col gap-1 bg-surface-0 dark:bg-surface-900 border-surface-200 dark:border-surface-800"
      >
        <div class="flex justify-between items-center text-xs mb-1">
          <span class="font-bold text-surface-900 dark:text-white">{{ event.event_type }}</span>
          <span class="text-surface-500">
            {{ formatTime(event.created_at) }}
          </span>
        </div>
        <span
          v-if="event.action_taken"
          class="text-xs inline-block px-1.5 py-0.5 rounded w-max bg-surface-100 dark:bg-surface-800 text-surface-600 dark:text-surface-400 font-medium"
        >
          {{ event.action_taken }}
        </span>
        <pre
          class="text-xs whitespace-pre-wrap font-mono overflow-x-auto mt-2 text-surface-600 dark:text-surface-400 bg-surface-50 dark:bg-surface-950 p-2 rounded"
          >{{ formatMetadata(event.payload) }}</pre
        >
      </div>
    </div>
  </div>
</template>
