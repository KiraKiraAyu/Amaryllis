<script setup lang="ts">
import { ref, computed, watch, nextTick } from "vue"
import Card from "primevue/card"
import type { FeedMessage } from "@/composables/useTraderDetail"
import { formatTime } from "@/utils/format"
import FeedMessages from "./FeedMessages.vue"
import FeedMessageTrader from "./FeedMessageTrader.vue"
import FeedMessagePrompt from "./FeedMessagePrompt.vue"
import FeedMessageCard from "./FeedMessageCard.vue"

const props = defineProps<{
  messages: FeedMessage[]
  filteredMessages: FeedMessage[]
  typing: boolean
  showSystemOps: boolean
}>()

const emit = defineEmits<{
  "update:showSystemOps": [value: boolean]
}>()

const selectedId = ref<string | null>(null)
const listContainer = ref<HTMLElement | null>(null)
const detailContainer = ref<HTMLElement | null>(null)

const selectedMessage = computed(
  () =>
    props.filteredMessages.find((m) => m.id === selectedId.value) ?? null,
)

// Ensure a valid selection when the filtered list changes
watch(
  () => props.filteredMessages,
  (msgs) => {
    if (selectedId.value && !msgs.some((m) => m.id === selectedId.value)) {
      selectedId.value = msgs.length > 0 ? msgs[msgs.length - 1].id : null
    }
    if (!selectedId.value && msgs.length > 0) {
      selectedId.value = msgs[msgs.length - 1].id
    }
  },
  { immediate: true },
)

// Auto-select streaming message + auto-scroll list to bottom
watch(
  () => props.messages.length,
  async () => {
    const last = props.messages[props.messages.length - 1]
    if (last?.streaming) {
      selectedId.value = last.id
    }
    await nextTick()
    if (listContainer.value) {
      listContainer.value.scrollTop = listContainer.value.scrollHeight
    }
  },
)

// Auto-scroll detail panel while streaming
watch(
  () => selectedMessage.value?.content?.length,
  async () => {
    if (selectedMessage.value?.streaming) {
      await nextTick()
      if (detailContainer.value) {
        detailContainer.value.scrollTop = detailContainer.value.scrollHeight
      }
    }
  },
)

function roleLabel(role: string): string {
  switch (role) {
    case "trader":
      return "Trader"
    case "prompt":
      return "System Prompt"
    case "notice":
      return "Notice"
    case "warning":
      return "Warning"
    default:
      return "System Ops"
  }
}
</script>

<template>
  <Card
    class="flex-1 min-h-0 border border-surface-200 dark:border-surface-800 bg-surface-0 dark:bg-surface-900 shadow-none!"
  >
    <template #content>
      <div class="flex flex-col h-full max-h-[calc(100vh-380px)]">
        <!-- Toolbar -->
        <div
          class="flex items-center justify-between pb-3 mb-1 border-b border-surface-200 dark:border-surface-800"
        >
          <span class="text-sm font-bold text-surface-600 dark:text-surface-400"
            >Activity Timeline</span
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

        <!-- Split layout -->
        <div class="flex gap-3 flex-1 min-h-0">
          <!-- Left: Event list -->
          <div
            ref="listContainer"
            class="w-60 shrink-0 overflow-y-auto border-r border-surface-200 dark:border-surface-800 pr-1"
          >
            <!-- Empty state -->
            <div
              v-if="props.messages.length === 0"
              class="flex flex-col items-center justify-center h-full gap-3 text-surface-400"
            >
              <span class="pi pi-comments text-3xl"></span>
              <p class="text-xs text-center">No activity yet</p>
            </div>

            <!-- List items -->
            <FeedMessages
              v-for="msg in props.filteredMessages"
              :key="msg.id"
              :msg="msg"
              :selected="msg.id === selectedId"
              @select="selectedId = msg.id"
            />

            <!-- Typing indicator -->
            <div
              v-if="props.typing"
              class="flex items-center gap-2 text-surface-400 px-3 py-2"
            >
              <span class="pi pi-spin pi-spinner text-xs"></span>
              <span class="text-xs font-medium">AI is thinking...</span>
            </div>
          </div>

          <!-- Right: Detail panel -->
          <div
            ref="detailContainer"
            class="flex-1 min-w-0 overflow-y-auto"
          >
            <!-- No selection -->
            <div
              v-if="!selectedMessage"
              class="flex items-center justify-center h-full text-surface-400"
            >
              <span class="text-sm">Select an event to view details</span>
            </div>

            <!-- Detail content -->
            <template v-else>
              <!-- Common header -->
              <div
                class="flex items-center gap-2 pb-3 mb-3 border-b border-surface-200 dark:border-surface-800"
              >
                <span class="text-xs font-mono text-surface-400">{{
                  formatTime(selectedMessage.timestamp)
                }}</span>
                <span
                  class="text-sm font-bold text-surface-700 dark:text-surface-200"
                >
                  {{ roleLabel(selectedMessage.role) }}
                </span>
              </div>

              <!-- Type-specific detail -->
              <FeedMessageTrader
                v-if="selectedMessage.role === 'trader'"
                :msg="selectedMessage"
              />
              <FeedMessagePrompt
                v-else-if="selectedMessage.role === 'prompt'"
                :msg="selectedMessage"
              />
              <FeedMessageCard v-else :msg="selectedMessage" />
            </template>
          </div>
        </div>
      </div>
    </template>
  </Card>
</template>
