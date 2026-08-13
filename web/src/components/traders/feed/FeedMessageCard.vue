<script setup lang="ts">
import type { FeedMessage } from "@/composables/useTraderDetail"

const props = defineProps<{
  msg: FeedMessage
}>()

function hasPayload(data: unknown): boolean {
  return (
    !!data && typeof data === "object" && Object.keys(data as object).length > 0
  )
}
</script>

<template>
  <div>
    <p
      v-if="props.msg.title"
      class="text-sm font-medium text-surface-700 dark:text-surface-200 mb-2"
    >
      {{ props.msg.title }}
    </p>
    <p
      v-if="props.msg.content"
      class="text-sm text-surface-600 dark:text-surface-300 whitespace-pre-wrap wrap-break-words mb-2"
    >
      {{ props.msg.content }}
    </p>
    <pre
      v-if="hasPayload(props.msg.data?.payload)"
      class="text-xs mt-2 p-2 rounded-lg overflow-auto max-h-48 font-mono bg-surface-100 dark:bg-surface-800 text-surface-600 dark:text-surface-400"
    >{{ JSON.stringify(props.msg.data?.payload, null, 2) }}</pre>
  </div>
</template>
