<script setup lang="ts">
import Button from "primevue/button"
import InputText from "primevue/inputtext"
import Select from "primevue/select"
import ToggleSwitch from "primevue/toggleswitch"
import type { ApiCategoryOption, LlmProvider } from "@/types/ai-models-ui"

defineProps<{
  provider: LlmProvider
  apiCategories: ApiCategoryOption[]
  checkingProvider: boolean
  savingModels: boolean
  checkMessage: string
}>()

const emit = defineEmits<{
  checkProvider: [provider: LlmProvider]
  removeProvider: []
  saveModels: []
}>()
</script>

<template>
  <div class="flex flex-col gap-4">
    <div class="flex flex-col gap-1">
      <label class="text-sm font-medium text-surface-700 dark:text-surface-300"
        >Provider Name</label
      >
      <InputText v-model="provider.name" />
    </div>

    <div class="flex flex-col gap-1">
      <label class="text-sm font-medium text-surface-700 dark:text-surface-300"
        >API Category</label
      >
      <Select
        v-model="provider.providerType"
        :options="apiCategories"
        optionLabel="label"
        optionValue="value"
        class="w-full"
      />
    </div>

    <div class="flex flex-col gap-1">
      <label class="text-sm font-medium text-surface-700 dark:text-surface-300"
        >API URL</label
      >
      <InputText
        v-model="provider.baseUrl"
        placeholder="https://api.example.com/v1"
      />
    </div>

    <div class="flex items-end gap-2">
      <div class="flex flex-col gap-1 flex-1">
        <label
          class="text-sm font-medium text-surface-700 dark:text-surface-300"
          >API Key</label
        >
        <InputText
          v-model="provider.apiKey"
          type="password"
          placeholder="sk-..."
        />
      </div>
      <Button
        :label="checkingProvider ? 'Testing...' : 'Test'"
        icon="pi pi-check"
        severity="secondary"
        variant="outlined"
        :disabled="checkingProvider || !provider.apiKey"
        @click="emit('checkProvider', provider)"
      />
    </div>

    <p v-if="checkMessage" class="px-1 text-xs font-medium text-surface-500">
      {{ checkMessage }}
    </p>

    <div class="mt-2 flex items-center justify-between">
      <div class="flex items-center gap-2">
        <ToggleSwitch v-model="provider.enabled" />
        <span class="text-sm font-medium text-surface-700 dark:text-surface-300"
          >Enabled</span
        >
      </div>

      <div class="flex gap-2">
        <Button
          label="Delete"
          icon="pi pi-trash"
          severity="danger"
          variant="text"
          @click="emit('removeProvider')"
        />
        <Button
          :label="savingModels ? 'Saving...' : 'Save'"
          icon="pi pi-save"
          :disabled="savingModels"
          @click="emit('saveModels')"
        />
      </div>
    </div>
  </div>
</template>
