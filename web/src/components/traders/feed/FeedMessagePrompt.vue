<script setup lang="ts">
import type { FeedMessage } from "@/composables/useTraderDetail"

const props = defineProps<{
  msg: FeedMessage
}>()
</script>

<template>
  <div
    class="max-w-[75%] rounded-3xl px-5 py-3.5 flex flex-col gap-2 bg-primary-500 text-white shadow-sm"
  >
    <p
      v-if="props.msg.content"
      class="text-sm text-white/95 whitespace-pre-wrap wrap-break-words leading-relaxed"
    >
      {{ props.msg.content }}
    </p>
  </div>
</template>
