<script setup lang="ts">
import { Icon } from "@iconify/vue"
import type { AvailableModelPayload } from "@/types/models"

defineProps<{
  remoteModels: AvailableModelPayload[]
  hasModel: (modelId: string) => boolean
}>()

const emit = defineEmits<{
  addRemoteModel: [model: AvailableModelPayload]
}>()
</script>

<template>
  <div
    v-if="remoteModels.length > 0"
    class="mb-4 rounded-md border border-border-subtle bg-surface p-3"
  >
    <div class="mb-2 text-xs font-semibold text-text-secondary">
      Available Models
    </div>
    <div class="flex max-h-48 flex-col gap-1 overflow-y-auto">
      <button
        v-for="remoteModel in remoteModels"
        :key="remoteModel.id"
        type="button"
        class="flex items-center gap-2 rounded-md px-2 py-1.5 text-left text-xs transition-colors hover:bg-surface-elevated"
        :class="
          hasModel(remoteModel.id)
            ? 'text-text-muted'
            : 'text-text-secondary'
        "
        :disabled="hasModel(remoteModel.id)"
        @click="emit('addRemoteModel', remoteModel)"
      >
        <Icon
          :icon="hasModel(remoteModel.id) ? 'ic:round-check' : 'ic:round-add'"
          class="shrink-0 text-base"
        />
        <span class="min-w-0 flex-1">
          <span class="block truncate font-medium">
            {{ remoteModel.name }}
          </span>
          <span class="block truncate text-text-muted">
            {{ remoteModel.id }}
          </span>
        </span>
      </button>
    </div>
  </div>
</template>
