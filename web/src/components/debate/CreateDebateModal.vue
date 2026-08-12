<script setup lang="ts">
import Dialog from "primevue/dialog"
import InputText from "primevue/inputtext"
import InputNumber from "primevue/inputnumber"
import Button from "primevue/button"
import type { DebateDraft } from "@/types/debate-ui"

defineProps<{
  personalities: string[]
  creating: boolean
  personalityEmoji: (personality: string) => string
}>()

const draft = defineModel<DebateDraft>({ required: true })

const emit = defineEmits<{
  create: []
  close: []
  togglePersonality: [personality: string]
}>()
</script>

<template>
  <Dialog
    visible
    modal
    header="Create AI Debate Session"
    class="w-full max-w-md border border-surface-200 dark:border-surface-800 bg-surface-0 dark:bg-surface-900 shadow-xl rounded-2xl p-6"
    :closable="true"
    @update:visible="emit('close')"
  >
    <div class="flex flex-col gap-4 mt-3">
      <div class="flex flex-col gap-1.5">
        <label class="text-xs font-bold text-surface-500">Debate Name</label>
        <InputText
          v-model="draft.name"
          placeholder="BTC Bull/Bear Debate"
          class="h-10 rounded-xl"
        />
      </div>

      <div class="flex flex-col gap-1.5">
        <label class="text-xs font-bold text-surface-500">Symbol</label>
        <InputText
          v-model="draft.symbol"
          placeholder="BTCUSDT"
          class="h-10 rounded-xl font-mono"
        />
      </div>

      <div class="flex flex-col gap-1.5">
        <label class="text-xs font-bold text-surface-500">Max Rounds</label>
        <InputNumber
          v-model="draft.max_rounds"
          :min="1"
          :max="10"
          showButtons
          class="h-10 rounded-xl"
        />
      </div>

      <div class="flex flex-col gap-1.5">
        <label class="text-xs font-bold text-surface-500 mb-1"
          >Participants</label
        >
        <div class="flex flex-wrap gap-2">
          <Button
            v-for="personality in personalities"
            :key="personality"
            @click="emit('togglePersonality', personality)"
            :label="personalityEmoji(personality) + ' ' + personality"
            text
            size="small"
            class="px-3 py-1.5 rounded-lg text-xs font-semibold! cursor-pointer"
            :class="
              draft.participants.includes(personality)
                ? 'bg-primary! text-primary-contrast!'
                : 'bg-surface-50 dark:bg-surface-950 text-surface-600 dark:text-surface-400 border border-surface-200 dark:border-surface-800 hover:text-primary'
            "
          />
        </div>
      </div>

      <div
        class="flex gap-3 mt-4 border-t border-surface-200 dark:border-surface-800 pt-4"
      >
        <Button
          label="Create"
          icon="pi pi-check"
          :loading="creating"
          @click="emit('create')"
          class="flex-1 rounded-xl h-11 cursor-pointer"
        />
        <Button
          label="Cancel"
          icon="pi pi-times"
          severity="secondary"
          text
          @click="emit('close')"
          class="rounded-xl h-11 cursor-pointer"
        />
      </div>
    </div>
  </Dialog>
</template>
