import { describe, it, expect } from "vitest"
import { createPinia } from "pinia"

describe("Test Setup", () => {
  it("should run basic test", () => {
    expect(1 + 1).toBe(2)
  })

  it("should create pinia instance", () => {
    const pinia = createPinia()
    expect(pinia).toBeDefined()
  })
})
