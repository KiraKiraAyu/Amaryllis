<script setup lang="ts">
import Button from "primevue/button"
import type { TraderPayload } from "@/types/trading"

defineProps<{ trader: TraderPayload }>()
defineEmits(["start", "stop", "sync"])
</script>

<template>
  <div
    class="flex items-center gap-4 py-3 px-4 rounded-xl border border-surface-200 dark:border-surface-800 bg-surface-50 dark:bg-surface-950 hover:bg-surface-100/50 dark:hover:bg-surface-900/30 transition-all duration-200"
  >
    <!-- Status indicator -->
    <div class="relative flex h-3 w-3 shrink-0">
      <span
        v-if="trader.is_running"
        class="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"
      ></span>
      <span
        class="relative inline-flex rounded-full h-3 w-3 transition-colors"
        :class="trader.is_running ? 'bg-emerald-500' : 'bg-surface-300 dark:bg-surface-600'"
      ></span>
    </div>

    <!-- Info -->
    <div class="flex-1 min-w-0">
      <div class="text-sm font-bold text-surface-900 dark:text-white truncate">
        {{ trader.name || trader.id }}
      </div>
      <div class="text-xs text-surface-400 dark:text-surface-500 truncate mt-0.5 font-medium">
        {{ trader.ai_model_id }} <span class="mx-1">·</span> {{ trader.exchange_id ?? "Paper" }}
      </div>
    </div>

    <!-- Actions -->
    <div class="flex items-center gap-2 shrink-0">
      <span
        class="text-xs font-bold uppercase tracking-wider px-2 py-0.5 rounded-md"
        :class="
          trader.is_running
            ? 'bg-emerald-500/10 text-emerald-500 dark:bg-emerald-500/20'
            : 'bg-surface-100 text-surface-500 dark:bg-surface-800 dark:text-surface-400'
        "
      >
        {{ trader.is_running ? "Running" : "Stopped" }}
      </span>

      <!-- Sync balance -->
      <Button
        icon="pi pi-refresh"
        severity="secondary"
        text
        rounded
        @click="$emit('sync')"
        title="Sync balance"
        class="h-9 w-9 cursor-pointer"
      />

      <!-- Start / Stop -->
      <Button
        v-if="!trader.is_running"
        icon="pi pi-play"
        severity="success"
        rounded
        @click="$emit('start')"
        title="Start trader"
        class="h-9 w-9 cursor-pointer bg-emerald-500! border-emerald-500! hover:bg-emerald-600! hover:border-emerald-600! text-white!"
      />
      <Button
        v-else
        icon="pi pi-stop"
        severity="danger"
        rounded
        @click="$emit('stop')"
        title="Stop trader"
        class="h-9 w-9 cursor-pointer bg-rose-500! border-rose-500! hover:bg-rose-600! hover:border-rose-600! text-white!"
      />
    </div>
  </div>
</template>
