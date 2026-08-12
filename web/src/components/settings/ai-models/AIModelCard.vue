<script setup lang="ts">
import Button from "primevue/button"
import type { LlmModel } from "@/types/ai-models-ui"

defineProps<{
  model: LlmModel
}>()

const emit = defineEmits<{
  editModel: []
  removeModel: []
}>()
</script>

<template>
  <div
    class="flex items-center gap-3 rounded-full bg-surface-50 dark:bg-surface-800 px-3 py-2"
  >
    <div
      class="flex-1 ml-4 truncate text-sm font-medium text-surface-900 dark:text-surface-0"
    >
      {{ model.name || model.modelId }}
    </div>

    <div class="flex shrink-0 gap-1">
      <Button
        icon="pi pi-pencil"
        severity="secondary"
        variant="text"
        v-tooltip.top="'Edit model'"
        @click="emit('editModel')"
      />
      <Button
        icon="pi pi-trash"
        severity="danger"
        variant="text"
        v-tooltip.top="'Delete model'"
        @click="emit('removeModel')"
      />
    </div>
  </div>
</template>
