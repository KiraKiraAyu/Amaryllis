<script setup lang="ts">
import Button from "primevue/button"
import type { LlmProvider } from "@/types/ai-models-ui"

defineProps<{
  providers: LlmProvider[]
  selectedProviderIndex: number
  supportedProviderTypeCount: number
  apiCategoryLabel: (providerType: string) => string
  providerKey: (provider: LlmProvider, index: number) => string
  providerLabel: (provider: LlmProvider) => string
}>()

const emit = defineEmits<{
  addProvider: []
  selectProvider: [index: number]
}>()
</script>

<template>
  <div
    class="flex w-64 shrink-0 flex-col gap-3 border-r border-surface-200 dark:border-surface-800 pr-4"
  >
    <Button
      label="Add Provider"
      icon="pi pi-plus"
      class="w-full"
      :disabled="supportedProviderTypeCount === 0"
      @click="emit('addProvider')"
    />

    <div class="flex flex-col gap-1">
      <button
        v-for="(provider, providerIndex) in providers"
        :key="providerKey(provider, providerIndex)"
        type="button"
        class="cursor-pointer rounded-xl px-4 py-3 text-left text-sm transition-colors hover:bg-surface-100 dark:hover:bg-surface-800"
        :class="
          selectedProviderIndex === providerIndex
            ? 'bg-surface-100 dark:bg-surface-800 text-surface-900 dark:text-white font-bold'
            : 'text-surface-700 dark:text-surface-300 font-medium'
        "
        @click="emit('selectProvider', providerIndex)"
      >
        <span class="block truncate">
          {{ providerLabel(provider) }}
        </span>
        <span
          class="block truncate text-xs text-surface-500 font-medium tracking-wide mt-1"
        >
          {{ apiCategoryLabel(provider.providerType) }} · {{ provider.baseUrl }}
        </span>
      </button>
    </div>
  </div>
</template>
