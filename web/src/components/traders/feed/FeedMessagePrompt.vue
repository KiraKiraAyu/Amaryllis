<script setup lang="ts">
import type { FeedMessage } from "@/composables/useTraderDetail"

const props = defineProps<{
  msg: FeedMessage
}>()
</script>

<template>
  <div>
    <pre
      v-if="props.msg.content"
      class="text-xs font-mono p-3 rounded-lg bg-surface-100 dark:bg-surface-800 text-surface-700 dark:text-surface-200 whitespace-pre-wrap wrap-break-words overflow-x-auto leading-relaxed"
    >{{ props.msg.content }}</pre>
  </div>
</template>
