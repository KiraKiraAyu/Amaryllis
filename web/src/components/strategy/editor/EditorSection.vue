<script setup lang="ts">
defineProps<{
  icon: string
  title: string
}>()
</script>

<template>
  <section
    class="rounded-2xl border border-surface-200 dark:border-surface-800 bg-surface-0 dark:bg-surface-900 shadow-none! p-5"
  >
    <div class="mb-4 flex items-center gap-3">
      <span
        class="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-primary-500/10 dark:bg-primary-500/20 text-primary"
      >
        <span :class="['pi', icon]"></span>
      </span>
      <div class="min-w-0 flex-1">
        <h3 class="font-bold text-sm text-surface-900 dark:text-white">
          {{ title }}
        </h3>
      </div>
      <slot name="actions" />
    </div>
    <slot />
  </section>
</template>
