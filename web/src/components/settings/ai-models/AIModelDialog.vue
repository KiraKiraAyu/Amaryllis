<script setup lang="ts">
import { computed, ref, watch } from "vue"
import Button from "primevue/button"
import Dialog from "primevue/dialog"
import InputText from "primevue/inputtext"
import type { LlmModel } from "@/types/ai-models-ui"

const props = defineProps<{
  open: boolean
  model: LlmModel
  isNew: boolean
}>()

const emit = defineEmits<{
  "update:open": [value: boolean]
  save: []
}>()

const displayNameEdited = ref(true)

watch(
  () => props.open,
  (open) => {
    if (open) {
      displayNameEdited.value = !props.isNew
    }
  },
)

watch(
  () => props.model.modelId,
  (newModelId) => {
    if (!displayNameEdited.value) {
      props.model.name = newModelId
    }
  },
)

const canSave = computed(
  () => !!props.model.name.trim() && !!props.model.modelId.trim(),
)

function close() {
  emit("update:open", false)
}
</script>

<template>
  <Dialog
    :visible="open"
    modal
    :header="isNew ? 'Add Model' : 'Edit Model'"
    :style="{ width: '25rem' }"
    @update:visible="emit('update:open', $event)"
  >
    <div class="flex flex-col gap-4 py-2">
      <div class="flex flex-col gap-1">
        <label
          class="text-sm font-medium text-surface-700 dark:text-surface-300"
          >Model ID</label
        >
        <InputText v-model="model.modelId" placeholder="e.g. gpt-5.6-sol" />
      </div>

      <div class="flex flex-col gap-1">
        <label
          class="text-sm font-medium text-surface-700 dark:text-surface-300"
          >Display Name</label
        >
        <InputText
          v-model="model.name"
          placeholder="e.g. GPT-5.6 Sol"
          @input="displayNameEdited = true"
        />
      </div>
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
          label="Save"
          icon="pi pi-check"
          :disabled="!canSave"
          @click="emit('save')"
        />
      </div>
    </template>
  </Dialog>
</template>
