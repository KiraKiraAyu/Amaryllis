<script setup lang="ts">
import { computed, watch } from "vue"
import Button from "primevue/button"
import Dialog from "primevue/dialog"
import Select from "primevue/select"
import type { LlmProvider } from "@/types/ai-models-ui"

const props = defineProps<{
  open: boolean
  provider: LlmProvider | undefined
  selectedModelId: string
  checking: boolean
}>()

const emit = defineEmits<{
  "update:open": [value: boolean]
  "update:selectedModelId": [value: string]
  confirm: []
}>()

const modelOptions = computed(() => {
  return (
    props.provider?.models
      .map((model) => ({
        label: `${model.name || model.modelId} (${model.modelId.trim()})`,
        value: model.modelId.trim(),
      }))
      .filter((model) => model.value) ?? []
  )
})

watch(
  () => props.open,
  (open) => {
    if (!open || props.selectedModelId || modelOptions.value.length === 0) {
      return
    }
    const firstModel = modelOptions.value[0]
    if (firstModel) {
      emit("update:selectedModelId", firstModel.value)
    }
  },
)

function close() {
  emit("update:open", false)
}

function updateSelectedModel(value: string) {
  emit("update:selectedModelId", value)
}
</script>

<template>
  <Dialog
    :visible="open"
    modal
    header="Test Provider"
    :style="{ width: '25rem' }"
    @update:visible="emit('update:open', $event)"
  >
    <div class="flex flex-col gap-4 py-2">
      <p class="text-sm text-surface-600 dark:text-surface-400 m-0">
        Select a model to send a minimal test request with this provider.
      </p>

      <div class="flex flex-col gap-1">
        <label
          class="text-sm font-medium text-surface-700 dark:text-surface-300"
          >Test Model</label
        >
        <Select
          :model-value="selectedModelId"
          :options="modelOptions"
          option-label="label"
          option-value="value"
          :disabled="modelOptions.length === 0"
          placeholder="Select a model"
          class="w-full"
          @update:model-value="updateSelectedModel"
        />
      </div>

      <p v-if="modelOptions.length === 0" class="text-xs text-surface-500 m-0">
        Add at least one model before testing this provider.
      </p>
    </div>

    <template #footer>
      <div class="flex justify-end gap-2">
        <Button
          label="Cancel"
          icon="pi pi-times"
          severity="secondary"
          variant="text"
          @click="close"
        />
        <Button
          :label="checking ? 'Testing...' : 'Test'"
          :icon="checking ? 'pi pi-spin pi-spinner' : 'pi pi-check'"
          :disabled="checking || !selectedModelId"
          @click="emit('confirm')"
        />
      </div>
    </template>
  </Dialog>
</template>
