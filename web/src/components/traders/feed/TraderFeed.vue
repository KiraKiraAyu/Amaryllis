<script setup lang="ts">
import { ref, watch, nextTick } from "vue"
import Card from "primevue/card"
import type { FeedMessage } from "@/composables/useTraderDetail"
import FeedMessages from "./FeedMessages.vue"

const props = defineProps<{
  messages: FeedMessage[]
  filteredMessages: FeedMessage[]
  typing: boolean
  showSystemOps: boolean
}>()

const emit = defineEmits<{
  "update:showSystemOps": [value: boolean]
}>()

const feedContainer = ref<HTMLElement | null>(null)

// Auto-scroll to bottom on new messages
watch(
  () => props.messages.length,
  async () => {
    await nextTick()
    if (feedContainer.value) {
      feedContainer.value.scrollTop = feedContainer.value.scrollHeight
    }
  },
)
</script>

<template>
  <Card
    class="flex-1 min-h-0 border border-surface-200 dark:border-surface-800 bg-surface-0 dark:bg-surface-900 shadow-none!"
  >
    <template #content>
      <div class="flex flex-col h-full max-h-[calc(100vh-380px)]">
        <!-- Feed toolbar -->
        <div
          class="flex items-center justify-between pb-3 mb-1 border-b border-surface-200 dark:border-surface-800"
        >
          <span class="text-sm font-bold text-surface-600 dark:text-surface-400"
            >Activity Feed</span
          >
          <button
            class="flex items-center gap-1.5 text-xs font-medium rounded-lg px-2.5 py-1 cursor-pointer transition-colors"
            :class="
              props.showSystemOps
                ? 'bg-primary-500/15 text-primary-600 dark:text-primary-400'
                : 'text-surface-400 hover:text-surface-600 dark:hover:text-surface-300 hover:bg-surface-100 dark:hover:bg-surface-800'
            "
            @click="emit('update:showSystemOps', !props.showSystemOps)"
          >
            <span
              class="pi text-sm"
              :class="props.showSystemOps ? 'pi-eye' : 'pi-eye-slash'"
            ></span>
            <span>System Ops</span>
          </button>
        </div>

        <!-- Scrollable feed -->
        <div
          ref="feedContainer"
          class="flex flex-col gap-3 overflow-y-auto flex-1 p-1"
        >
          <!-- Empty state -->
          <div
            v-if="props.messages.length === 0"
            class="flex flex-col items-center justify-center h-full gap-3 text-surface-400"
          >
            <span class="pi pi-comments text-4xl"></span>
            <p class="text-sm">
              No activity yet. Start the trader to see live AI decisions
              and actions.
            </p>
          </div>

          <!-- Messages -->
          <FeedMessages
            v-for="msg in props.filteredMessages"
            :key="msg.id"
            :msg="msg"
          />

          <!-- Typing indicator -->
          <div
            v-if="props.typing"
            class="flex items-center gap-2 text-surface-400 pl-2"
          >
            <span class="pi pi-spin pi-spinner text-sm"></span>
            <span class="text-xs font-medium">AI is thinking...</span>
          </div>
        </div>
      </div>
    </template>
  </Card>
</template>
