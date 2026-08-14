<script setup lang="ts">
import InputText from "primevue/inputtext"
import InputNumber from "primevue/inputnumber"
import { computed } from "vue"
import type { EditableStrategy } from "@/types/strategy-ui"
import type { StrategyConfigPayload } from "@/types/strategies"
import EditorSection from "./EditorSection.vue"

const props = defineProps<{
  strategy: EditableStrategy
  config: StrategyConfigPayload
}>()

const strategy = computed(() => props.strategy)
const config = computed(() => props.config)
</script>

<template>
  <EditorSection
    icon="pi-pencil"
    title="Basic Information"
    description="Name, description, and global strategy parameters"
  >
    <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
      <div class="flex flex-col gap-1.5">
        <label
          class="text-xs font-bold uppercase tracking-wider text-surface-400 dark:text-surface-500"
          >Strategy Name</label
        >
        <InputText
          v-model="strategy.name"
          placeholder="Strategy name"
          class="h-10 rounded-xl"
        />
      </div>
      <div class="flex flex-col gap-1.5">
        <label
          class="text-xs font-bold uppercase tracking-wider text-surface-400 dark:text-surface-500"
          >Description</label
        >
        <InputText
          v-model="strategy.description"
          placeholder="Brief description"
          class="h-10 rounded-xl"
        />
      </div>
      <div class="flex flex-col gap-1.5">
        <label
          class="text-xs font-bold uppercase tracking-wider text-surface-400 dark:text-surface-500"
          >Max Positions</label
        >
        <InputNumber
          v-model="config.max_positions"
          :min="1"
          :max="20"
          showButtons
          class="h-10 rounded-xl"
        />
      </div>
    </div>
  </EditorSection>
</template>
