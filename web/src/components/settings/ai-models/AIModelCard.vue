<script setup lang="ts">
import Button from "primevue/button"
import InputText from "primevue/inputtext"
import ToggleSwitch from "primevue/toggleswitch"
import type { LlmModel } from "@/types/ai-models-ui"

defineProps<{
  model: LlmModel
}>()

const emit = defineEmits<{
  saveModels: []
  removeModel: []
}>()
</script>

<template>
  <div class="mb-4 rounded-md border border-surface-200 dark:border-surface-700 bg-surface-50 dark:bg-surface-800 p-4">
    <div class="mb-3 flex items-center justify-between">
      <div class="flex items-center gap-2">
        <ToggleSwitch v-model="model.enabled" />
        <span class="text-sm font-medium text-surface-700 dark:text-surface-300">Enabled</span>
      </div>
      <div class="flex gap-1">
        <Button
          icon="pi pi-save"
          severity="secondary"
          variant="text"
          v-tooltip.top="'Save changes'"
          @click="emit('saveModels')"
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
    
    <div class="flex flex-col gap-3">
      <div class="flex flex-col gap-1">
        <label class="text-sm font-medium text-surface-700 dark:text-surface-300">Display Name</label>
        <InputText
          v-model="model.name"
          placeholder="GPT-5.6 Sol"
        />
      </div>
      <div class="flex flex-col gap-1">
        <label class="text-sm font-medium text-surface-700 dark:text-surface-300">Model ID</label>
        <InputText
          v-model="model.modelId"
          placeholder="gpt-5.6-sol"
        />
      </div>
    </div>
  </div>
</template>
