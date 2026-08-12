import { describe, it, expect, vi, beforeEach } from "vitest"
import axios from "axios"

vi.mock("axios")

describe("API Client", () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it("should make GET request", async () => {
    const mockData = { data: "test" }
    vi.mocked(axios.get).mockResolvedValue({ data: mockData })

    const response = await axios.get("/api/test")

    expect(response.data).toEqual(mockData)
    expect(axios.get).toHaveBeenCalledWith("/api/test")
  })

  it("should handle API errors", async () => {
    const mockError = new Error("Network error")
    vi.mocked(axios.get).mockRejectedValue(mockError)

    await expect(axios.get("/api/test")).rejects.toThrow("Network error")
  })

  it("should make POST request with data", async () => {
    const mockData = { success: true }
    const postData = { name: "test" }
    vi.mocked(axios.post).mockResolvedValue({ data: mockData })

    const response = await axios.post("/api/test", postData)

    expect(response.data).toEqual(mockData)
    expect(axios.post).toHaveBeenCalledWith("/api/test", postData)
  })
})
