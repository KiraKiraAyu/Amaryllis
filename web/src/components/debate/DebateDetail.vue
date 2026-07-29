<script setup lang="ts">
import Button from "primevue/button"
import type { DebateMessage, DebateSession } from "@/types/debate-ui"

defineProps<{
  debate: DebateSession
  messages: DebateMessage[]
}>()

const emit = defineEmits<{
  start: [id: string]
  cancel: [id: string]
}>()

function personalityEmoji(personality: string) {
  const icons: Record<string, string> = {
    bull: "🐂",
    bear: "🐻",
    analyst: "🔬",
    contrarian: "🎭",
    risk_manager: "🛡️",
  }
  return icons[personality] ?? "🤖"
}

function msgStyle(personality: string) {
  const colors: Record<string, string> = {
    bull: "oklch(0.72 0.17 145 / 0.06)",
    bear: "oklch(0.65 0.21 15 / 0.06)",
    analyst: "oklch(0.66 0.058 301 / 0.07)",
    contrarian: "oklch(0.82 0.16 85 / 0.06)",
    risk_manager: "oklch(0.65 0.21 236 / 0.06)",
  }
  const borders: Record<string, string> = {
    bull: "oklch(0.72 0.17 145 / 0.2)",
    bear: "oklch(0.65 0.21 15 / 0.2)",
    analyst: "oklch(0.66 0.058 301 / 0.25)",
    contrarian: "oklch(0.82 0.16 85 / 0.2)",
    risk_manager: "oklch(0.65 0.21 236 / 0.2)",
  }
  return `background-color:${colors[personality] ?? "var(--color-surface-elevated)"};border:1px solid ${borders[personality] ?? "var(--color-border-subtle)"}`
}
</script>

<template>
  <div class="flex flex-col gap-6 bg-surface-50 dark:bg-surface-900 border border-surface-200 dark:border-surface-800 rounded-2xl p-6 transition-colors duration-300">
    <!-- Header -->
    <div class="flex items-center justify-between border-b border-surface-200 dark:border-surface-800 pb-4 flex-wrap gap-4">
      <div>
        <h2 class="font-bold text-xl text-surface-900 dark:text-white">{{ debate.name || debate.symbol }}</h2>
        <div class="flex items-center gap-2 mt-1 text-xs text-surface-500 dark:text-surface-400">
          <span>Round {{ debate.current_round }}/{{ debate.max_rounds }}</span>
          <span>·</span>
          <span class="capitalize px-2 py-0.5 rounded-md font-semibold text-xs tracking-wider"
            :class="{
              'bg-emerald-100 text-emerald-700 dark:bg-emerald-950/30 dark:text-emerald-400': debate.status === 'running',
              'bg-orange-100 text-orange-700 dark:bg-orange-950/30 dark:text-orange-400': debate.status === 'pending',
              'bg-red-100 text-red-700 dark:bg-red-950/30 dark:text-red-400': debate.status === 'failed',
              'bg-surface-100 text-surface-700 dark:bg-surface-800 dark:text-surface-300': debate.status === 'finished'
            }"
          >
            {{ debate.status }}
          </span>
        </div>
      </div>
      <div>
        <Button
          v-if="debate.status === 'pending' || debate.status === 'failed'"
          icon="pi pi-play"
          label="Start"
          size="small"
          class="rounded-xl cursor-pointer"
          @click="emit('start', debate.id)"
        />
        <Button
          v-if="debate.status === 'running'"
          icon="pi pi-stop"
          label="Cancel"
          severity="danger"
          size="small"
          class="rounded-xl cursor-pointer"
          @click="emit('cancel', debate.id)"
        />
      </div>
    </div>

    <!-- Final Decision -->
    <div
      v-if="debate.final_decision"
      class="p-4 rounded-xl border border-primary-200/50 dark:border-primary-900/40 bg-primary-50/20 dark:bg-primary-950/10"
    >
      <h3 class="font-bold text-sm text-primary-600 dark:text-primary-400 mb-2 flex items-center gap-1.5">
        <span class="pi pi-verified text-sm"></span>
        Final Decision
      </h3>
      <div class="flex items-start gap-4">
        <span class="text-3xl font-black text-primary-600 dark:text-primary-400 select-none">{{ debate.final_decision }}</span>
        <span class="text-xs text-surface-600 dark:text-surface-400 leading-relaxed pt-1.5">
          {{ debate.final_reasoning }}
        </span>
      </div>
    </div>

    <!-- Debate Thread -->
    <div>
      <h3 class="font-bold text-sm text-surface-700 dark:text-surface-300 mb-3 flex items-center gap-2">
        <span class="pi pi-comments"></span>
        Debate Thread
      </h3>
      <div class="flex flex-col gap-3 max-h-125 overflow-auto pr-1">
        <div
          v-if="messages.length === 0"
          class="text-center py-12 text-sm text-surface-400 dark:text-surface-500 bg-surface-0 dark:bg-surface-950/30 border border-dashed border-surface-200 dark:border-surface-800 rounded-xl"
        >
          No messages yet. Start the debate to begin.
        </div>
        <div
          v-for="(message, index) in messages"
          :key="index"
          class="p-3.5 rounded-xl text-sm transition-all hover:translate-x-0.5"
          :style="msgStyle(message.personality)"
        >
          <div class="flex items-center gap-2 mb-1.5">
            <span class="text-base select-none">
              {{ personalityEmoji(message.personality) }}
            </span>
            <span class="font-bold capitalize text-surface-800 dark:text-surface-200">{{ message.personality }}</span>
            <span class="text-xs ml-auto text-surface-500 dark:text-surface-500 font-medium">
              Round {{ message.round }}
            </span>
            <span
              v-if="message.vote"
              class="text-xs font-black uppercase px-1.5 py-0.5 rounded border ml-1.5"
              :class="
                message.vote === 'BULLISH'
                  ? 'text-emerald-500 border-emerald-500/20 bg-emerald-500/5'
                  : 'text-rose-500 border-rose-500/20 bg-rose-500/5'
              "
            >
              {{ message.vote }}
            </span>
          </div>
          <p class="text-xs leading-relaxed text-surface-600 dark:text-surface-300">
            {{ message.content }}
          </p>
        </div>
      </div>
    </div>
  </div>
</template>

