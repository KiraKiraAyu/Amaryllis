<script setup lang="ts">
import { Icon } from "@iconify/vue"

const props = withDefaults(
  defineProps<{
    modelValue: boolean
    title?: string
    closeOnBackdrop?: boolean
  }>(),
  {
    title: "",
    closeOnBackdrop: true,
  },
)

const emit = defineEmits<{
  "update:modelValue": [value: boolean]
  close: []
}>()

function close() {
  emit("update:modelValue", false)
  emit("close")
}

function closeFromBackdrop() {
  if (props.closeOnBackdrop) close()
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="modelValue"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
      @click.self="closeFromBackdrop"
    >
      <div
        class="w-full max-w-md rounded-xl border border-border-subtle bg-surface p-5 shadow-2xl"
        role="dialog"
        aria-modal="true"
      >
        <div class="mb-4 flex items-center gap-3">
          <h2 class="min-w-0 flex-1 text-base font-semibold">
            {{ title }}
          </h2>
          <button
            type="button"
            class="rounded-full p-1 text-text-muted transition-colors hover:bg-surface-elevated hover:text-text-primary"
            title="Close"
            @click="close"
          >
            <Icon icon="ic:round-close" class="text-lg" />
          </button>
        </div>

        <slot />

        <div v-if="$slots.footer" class="mt-5 flex justify-end gap-2">
          <slot name="footer" />
        </div>
      </div>
    </div>
  </Teleport>
</template>
