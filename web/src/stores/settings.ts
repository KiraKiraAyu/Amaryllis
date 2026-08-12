import { defineStore } from "pinia"
import { ref } from "vue"
import { getSettingsApi, updateSettingsApi } from "@/api/settings"
import { detectBrowserTimeZone, setDefaultTimeZone } from "@/utils/format"

export const useSettingsStore = defineStore("settings", () => {
  const timezone = ref<string | null>(null)
  const initialized = ref(false)

  /**
   * Load settings from backend. Called on app init when logged in.
   * If no timezone is set yet, detects the browser timezone and saves it.
   */
  async function load() {
    try {
      const res = await getSettingsApi()
      timezone.value = res.timezone

      // First launch: no timezone saved yet → detect & persist
      if (!res.timezone) {
        const browserTz = detectBrowserTimeZone()
        if (browserTz) {
          await save(browserTz)
        }
      } else {
        setDefaultTimeZone(res.timezone)
      }
    } catch {
      // If settings API fails, fall back to browser timezone for display
      const browserTz = detectBrowserTimeZone()
      if (browserTz) {
        timezone.value = browserTz
        setDefaultTimeZone(browserTz)
      }
    } finally {
      initialized.value = true
    }
  }

  /** Save timezone to backend and update local state. */
  async function save(tz: string) {
    const res = await updateSettingsApi({ timezone: tz })
    timezone.value = res.timezone
    setDefaultTimeZone(res.timezone ?? undefined)
  }

  return {
    timezone,
    initialized,
    load,
    save,
  }
})
