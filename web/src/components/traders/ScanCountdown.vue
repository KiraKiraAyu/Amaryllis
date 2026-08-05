<script setup lang="ts">
import { ref, computed, watch, onUnmounted } from "vue"

const props = defineProps<{
  /** Unix timestamp (seconds) of the next scheduled scan, or null when scanning/stopped. */
  nextScanAt: number | null
  /** Whether the trader is currently running. */
  isRunning: boolean
}>()

const nowSecs = ref(Math.floor(Date.now() / 1000))
let timer: ReturnType<typeof setInterval> | null = null

function tick() {
  nowSecs.value = Math.floor(Date.now() / 1000)
}

watch(
  () => props.isRunning,
  (running) => {
    if (running) {
      tick()
      if (!timer) timer = setInterval(tick, 1000)
    } else {
      if (timer) {
        clearInterval(timer)
        timer = null
      }
    }
  },
  { immediate: true },
)

onUnmounted(() => {
  if (timer) clearInterval(timer)
})

/** Remaining seconds until the next scan. */
const remainingSecs = computed(() => {
  if (props.nextScanAt == null) return 0
  return Math.max(0, props.nextScanAt - nowSecs.value)
})

/** Whether the engine is currently scanning (running but no next_scan_at). */
const isScanning = computed(
  () => props.isRunning && props.nextScanAt == null,
)

/** Whether the countdown is active (running and has a target). */
const isActive = computed(
  () => props.isRunning && props.nextScanAt != null,
)

/** Format the remaining seconds as MM:SS or HH:MM:SS. */
const countdownText = computed(() => {
  const total = remainingSecs.value
  const hours = Math.floor(total / 3600)
  const minutes = Math.floor((total % 3600) / 60)
  const seconds = total % 60
  const pad = (n: number) => n.toString().padStart(2, "0")
  if (hours > 0) {
    return `${pad(hours)}:${pad(minutes)}:${pad(seconds)}`
  }
  return `${pad(minutes)}:${pad(seconds)}`
})
</script>

<template>
  <div v-if="isRunning" class="flex flex-col">
    <span class="text-xs text-surface-400 font-bold uppercase tracking-wider">
      Next Scan
    </span>
    <div class="flex items-center gap-1.5">
      <i
        v-if="isScanning"
        class="pi pi-spin pi-spinner text-xs text-primary-500"
      />
      <span
        v-if="isScanning"
        class="text-sm font-bold font-mono text-primary-500"
      >
        Scanning...
      </span>
      <span
        v-else-if="isActive"
        class="text-sm font-bold font-mono text-surface-900 dark:text-white tabular-nums"
      >
        {{ countdownText }}
      </span>
      <span v-else class="text-sm font-bold font-mono text-surface-400">--:--</span>
    </div>
  </div>
</template>
