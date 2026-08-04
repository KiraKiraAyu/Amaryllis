<script setup lang="ts">
import { ref, onMounted, computed } from "vue"
import Select from "primevue/select"
import Button from "primevue/button"
import { useSettingsStore } from "@/stores/settings"
import { useToast } from "primevue/usetoast"
import { detectBrowserTimeZone } from "@/utils/format"

const settings = useSettingsStore()
const toast = useToast()

const selectedTz = ref<string | null>(null)
const saving = ref(false)

/** Build a list of IANA timezones supported by the browser. */
const timezoneOptions = computed(() => {
  try {
    const list = Intl.supportedValuesOf("timeZone") as string[]
    return list.map((tz) => ({ label: tz, value: tz }))
  } catch {
    // Fallback: a small curated list
    return [
      "UTC",
      "America/New_York",
      "America/Chicago",
      "America/Los_Angeles",
      "Europe/London",
      "Europe/Paris",
      "Asia/Tokyo",
      "Asia/Shanghai",
      "Asia/Singapore",
    ].map((tz) => ({ label: tz, value: tz }))
  }
})

const browserTz = computed(() => detectBrowserTimeZone())

onMounted(() => {
  selectedTz.value = settings.timezone
})

async function save() {
  if (!selectedTz.value) return
  saving.value = true
  try {
    await settings.save(selectedTz.value)
    toast.add({
      severity: "success",
      summary: "Saved",
      detail: "Timezone updated",
      life: 3000,
    })
  } catch {
    toast.add({
      severity: "error",
      summary: "Error",
      detail: "Failed to save timezone",
      life: 5000,
    })
  } finally {
    saving.value = false
  }
}

function useBrowserTz() {
  if (browserTz.value) {
    selectedTz.value = browserTz.value
  }
}
</script>

<template>
  <div class="max-w-2xl flex flex-col gap-6">
    <!-- Timezone -->
    <div class="flex flex-col gap-2">
      <label class="text-sm font-semibold text-surface-700 dark:text-surface-300">
        Timezone
      </label>
      <p class="text-xs text-surface-500 dark:text-surface-400">
        Used for displaying all timestamps across the app.
      </p>
      <div class="flex items-center gap-3">
        <Select
          v-model="selectedTz"
          :options="timezoneOptions"
          optionLabel="label"
          optionValue="value"
          placeholder="Select timezone"
          filter
          :pt="{
            root: { class: 'flex-1' },
          }"
        />
        <Button
          label="Use Browser"
          severity="secondary"
          variant="outlined"
          size="small"
          @click="useBrowserTz"
        />
      </div>
      <div class="flex items-center gap-3 mt-2">
        <Button
          label="Save"
          :loading="saving"
          @click="save"
        />
        <span v-if="settings.timezone" class="text-xs text-surface-400">
          Current: {{ settings.timezone }}
        </span>
      </div>
    </div>
  </div>
</template>
