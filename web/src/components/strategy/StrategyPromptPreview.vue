<script setup lang="ts">
import Card from "primevue/card"
import Button from "primevue/button"
import type { StrategyPromptPreviewModel } from "@/types/strategy-ui"

defineProps<{
  preview: StrategyPromptPreviewModel
}>()

const emit = defineEmits<{
  close: []
}>()
</script>

<template>
  <Card
    class="border border-surface-200 dark:border-surface-800 bg-surface-50 dark:bg-surface-950 shadow-none! mb-4"
  >
    <template #content>
      <div class="flex items-center justify-between mb-3">
        <h3 class="font-bold text-sm text-surface-900 dark:text-white">
          Generated System Prompt
        </h3>
        <Button
          icon="pi pi-times"
          label="Close"
          severity="secondary"
          text
          size="small"
          @click="emit('close')"
          class="h-8 cursor-pointer"
        />
      </div>
      <pre
        class="whitespace-pre-wrap text-xs p-3 rounded-xl overflow-y-auto max-h-60 font-mono bg-surface-100 dark:bg-surface-900 text-surface-700 dark:text-surface-300 border border-surface-200 dark:border-surface-800"
        >{{ preview.system }}</pre
      >
      <div
        class="mt-3 text-xs text-surface-500 dark:text-surface-400 font-medium"
      >
        Variant:
        <span class="font-bold text-primary">{{ preview.variant }}</span>
      </div>
    </template>
  </Card>
</template>
