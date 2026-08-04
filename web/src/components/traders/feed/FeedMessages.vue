<script setup lang="ts">
import type { FeedMessage } from "@/composables/useTraderDetail"
import { formatTime } from "@/utils/format"
import FeedMessageTrader from "./FeedMessageTrader.vue"
import FeedMessagePrompt from "./FeedMessagePrompt.vue"
import FeedMessageCard from "./FeedMessageCard.vue"

const props = defineProps<{
  msg: FeedMessage
}>()

/** Whether a message should be right-aligned.
 *  Prompt (user → AI) and System Ops (action / system) are on the right;
 *  AI Trader and Position are on the left. */
function isRightSide(role: string): boolean {
  return role === "prompt" || role === "action" || role === "system"
}
</script>

<template>
  <div
    class="flex flex-col gap-1"
    :class="isRightSide(props.msg.role) ? 'items-end' : 'items-start'"
  >
    <FeedMessageTrader v-if="props.msg.role === 'trader'" :msg="props.msg" />
    <FeedMessagePrompt v-else-if="props.msg.role === 'prompt'" :msg="props.msg" />
    <FeedMessageCard v-else :msg="props.msg" />
    <span class="text-xs text-surface-400 font-mono px-1">
      {{ formatTime(props.msg.timestamp) }}
    </span>
  </div>
</template>
