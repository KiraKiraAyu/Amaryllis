import { describe, it, expect } from "vitest"
import {
  formatDate,
  formatDateTime,
  formatTime,
  fmtUsd,
  normalizeSide,
  isLongSide,
  setDefaultTimeZone,
  getDefaultTimeZone,
  detectBrowserTimeZone,
} from "@/utils/format"

describe("format utils", () => {
  it("formats backend second timestamps as dates", () => {
    expect(formatDate(1_700_000_000, { timeZone: "UTC" })).toBe("11/14/2023")
  })

  it("normalizes second and millisecond timestamps to the same instant", () => {
    const options = { timeZone: "UTC" }

    expect(formatDateTime(1_700_000_000, options)).toBe(
      formatDateTime(1_700_000_000_000, options),
    )
  })

  it("supports timestamp strings", () => {
    expect(formatDate("1700000000", { timeZone: "UTC" })).toBe("11/14/2023")
  })

  it("returns a fallback for invalid values", () => {
    expect(formatDate("not-a-date", { fallback: "Invalid" })).toBe("Invalid")
    expect(formatDate(null, { fallback: "N/A" })).toBe("N/A")
  })

  it("formats time correctly", () => {
    const timeStr = formatTime(1_700_000_000, { timeZone: "UTC" })
    expect(timeStr).toMatch(/\d{2}:\d{2}:\d{2}/)
  })

  it("formats USD currency values with two decimal places", () => {
    expect(fmtUsd(1234.56)).toBe("1,234.56")
    expect(fmtUsd(0)).toBe("0.00")
    expect(fmtUsd(null)).toBe("0.00")
    expect(fmtUsd(undefined)).toBe("0.00")
    expect(fmtUsd(NaN)).toBe("0.00")
    expect(fmtUsd(1000000)).toBe("1,000,000.00")
  })

  it("normalizes position sides correctly", () => {
    expect(normalizeSide("buy")).toBe("Long")
    expect(normalizeSide("LONG")).toBe("Long")
    expect(normalizeSide("sell")).toBe("Short")
    expect(normalizeSide("SHORT")).toBe("Short")
    expect(normalizeSide("custom")).toBe("custom")
  })

  it("identifies long sides correctly", () => {
    expect(isLongSide("buy")).toBe(true)
    expect(isLongSide("LONG")).toBe(true)
    expect(isLongSide("sell")).toBe(false)
    expect(isLongSide("short")).toBe(false)
  })

  it("sets and gets default timezone", () => {
    setDefaultTimeZone("America/New_York")
    expect(getDefaultTimeZone()).toBe("America/New_York")
    setDefaultTimeZone(undefined)
    expect(getDefaultTimeZone()).toBeUndefined()
  })

  it("detects browser timezone safely", () => {
    const tz = detectBrowserTimeZone()
    expect(typeof tz === "string" || tz === undefined).toBe(true)
  })
})
