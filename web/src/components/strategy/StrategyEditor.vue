<script setup lang="ts">
import Button from "primevue/button"
import { computed } from "vue"
import type { EditableStrategy } from "@/types/strategy-ui"
import StrategyBasicsSection from "./editor/StrategyBasicsSection.vue"
import StrategyDataSection from "./editor/StrategyDataSection.vue"
import StrategySymbolsSection from "./editor/StrategySymbolsSection.vue"
import StrategyTpSlSection from "./editor/StrategyTpSlSection.vue"
import { getDataTemplate, computeDataTemplateError } from "./editor/data-template"

const selected = defineModel<EditableStrategy>({ required: true })

defineProps<{
  saving: boolean
  duplicating: boolean
  testRunLoading?: boolean
}>()

const emit = defineEmits<{
  save: []
  cancel: []
  test: []
}>()

// Config is guaranteed to exist — ensureStrategyConfig runs in the composable
// before the editor renders. Pure read, no side effects.
const config = computed(() => selected.value.config)

const dataTemplateError = computed(() =>
  computeDataTemplateError(getDataTemplate(config.value).items),
)
</script>

<template>
  <div class="flex flex-col gap-5">
    <div class="flex items-center gap-3">
      <h2
        class="flex-1 truncate font-bold text-xl text-surface-900 dark:text-white"
      >
        Edit Strategy
      </h2>
    </div>

    <StrategyBasicsSection :strategy="selected" :config="config" />
    <StrategyDataSection :config="config" :strategy-id="selected.id" />
    <StrategySymbolsSection :config="config" />
    <StrategyTpSlSection :config="config" />

    <div class="flex flex-wrap items-center gap-3 pb-4">
      <p
        v-if="dataTemplateError"
        class="flex min-w-0 items-center gap-1.5 text-xs font-semibold text-rose-500"
      >
        <span class="pi pi-exclamation-circle shrink-0"></span>
        <span class="truncate">{{ dataTemplateError }}</span>
      </p>
      <div class="ml-auto flex gap-3">
        <Button
          icon="pi pi-save"
          label="Save"
          :disabled="!!dataTemplateError"
          :loading="saving"
          @click="emit('save')"
          class="h-11 cursor-pointer rounded-xl"
        />
        <Button
          icon="pi pi-sparkles"
          label="Test Run (AI)"
          severity="help"
          :disabled="!!dataTemplateError"
          :loading="testRunLoading"
          @click="emit('test')"
          class="h-11 cursor-pointer rounded-xl"
        />
        <Button
          icon="pi pi-times"
          label="Cancel"
          severity="secondary"
          @click="emit('cancel')"
          class="h-11 w-32 cursor-pointer rounded-xl"
        />
      </div>
    </div>
  </div>
</template>
