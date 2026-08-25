import { describe, it, expect, vi } from "vitest"
import ToastEventBus from "primevue/toasteventbus"
import { toast, useToast } from "@/utils/toast"

describe("Universal Toast Utility", () => {
  it("emits success message to ToastEventBus", () => {
    const spy = vi.spyOn(ToastEventBus, "emit")
    toast.success("Operation completed", "Custom Success", 2000)

    expect(spy).toHaveBeenCalledWith("add", {
      severity: "success",
      summary: "Custom Success",
      detail: "Operation completed",
      life: 2000,
    })
    spy.mockRestore()
  })

  it("emits error message with default values", () => {
    const spy = vi.spyOn(ToastEventBus, "emit")
    toast.error("Something went wrong")

    expect(spy).toHaveBeenCalledWith("add", {
      severity: "error",
      summary: "Error",
      detail: "Something went wrong",
      life: 5000,
    })
    spy.mockRestore()
  })

  it("emits warn and info messages correctly", () => {
    const spy = vi.spyOn(ToastEventBus, "emit")
    toast.warning("Warning text")
    expect(spy).toHaveBeenCalledWith("add", {
      severity: "warn",
      summary: "Warning",
      detail: "Warning text",
      life: 5000,
    })

    toast.info("Info text")
    expect(spy).toHaveBeenCalledWith("add", {
      severity: "info",
      summary: "Info",
      detail: "Info text",
      life: 3000,
    })
    spy.mockRestore()
  })

  it("supports useToast composable helper", () => {
    const t = useToast()
    expect(t).toBe(toast)
  })
})
