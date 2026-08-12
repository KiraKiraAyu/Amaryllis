import { defineStore } from "pinia"
import { ref } from "vue"

export interface ToastMessage {
  severity: "success" | "info" | "warn" | "error"
  summary: string
  detail: string
  life?: number
}

export const useToast = defineStore("toast", () => {
  const toastEvent = ref<ToastMessage | null>(null)

  const success = (message: string, summary = "Success") => {
    toastEvent.value = {
      severity: "success",
      summary,
      detail: message,
      life: 3000,
    }
  }

  const warning = (message: string, summary = "Warning") => {
    toastEvent.value = {
      severity: "warn",
      summary,
      detail: message,
      life: 5000,
    }
  }

  const error = (message: string, summary = "Error") => {
    toastEvent.value = {
      severity: "error",
      summary,
      detail: message,
      life: 5000,
    }
  }

  return { toastEvent, success, warning, error }
})
