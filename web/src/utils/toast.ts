import ToastEventBus from "primevue/toasteventbus"
import type { ToastMessageOptions } from "primevue/toast"

export const toast = {
  add(message: ToastMessageOptions) {
    ToastEventBus.emit("add", message)
  },

  success(detail: string, summary = "Success", life = 3000) {
    ToastEventBus.emit("add", {
      severity: "success",
      summary,
      detail,
      life,
    })
  },

  info(detail: string, summary = "Info", life = 3000) {
    ToastEventBus.emit("add", {
      severity: "info",
      summary,
      detail,
      life,
    })
  },

  warning(detail: string, summary = "Warning", life = 5000) {
    ToastEventBus.emit("add", {
      severity: "warn",
      summary,
      detail,
      life,
    })
  },

  error(detail: string, summary = "Error", life = 5000) {
    ToastEventBus.emit("add", {
      severity: "error",
      summary,
      detail,
      life,
    })
  },

  clear() {
    ToastEventBus.emit("remove-all-groups")
  },
}

/** Composable helper returning the universal toast instance. */
export function useToast() {
  return toast
}
