<script setup lang="ts">
import Button from "primevue/button"
import InputText from "primevue/inputtext"
import type { LlmModel } from "@/types/ai-models-ui"

defineProps<{
  model: LlmModel
}>()

const emit = defineEmits<{
  cancelAddModel: []
  saveNewModel: []
}>()
</script>

<template>
  <div class="mb-4 rounded-md border border-dashed border-surface-300 dark:border-surface-600 bg-surface-0 dark:bg-surface-900 p-4">
    <h4 class="mb-3 text-sm font-semibold text-surface-900 dark:text-surface-0">New Model</h4>
    
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
    
    <div class="mt-4 flex justify-end gap-2">
      <Button 
        label="Cancel" 
        icon="pi pi-times" 
        severity="secondary" 
        variant="outlined" 
        size="small"
        @click="emit('cancelAddModel')" 
      />
      <Button 
        label="Save" 
        icon="pi pi-check" 
        size="small"
        @click="emit('saveNewModel')" 
      />
    </div>
  </div>
</template>
