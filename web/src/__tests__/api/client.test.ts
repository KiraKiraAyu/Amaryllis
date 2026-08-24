import { describe, it, expect, vi, beforeEach } from "vitest"
import { setActivePinia, createPinia } from "pinia"
import axios from "axios"
import request from "@/utils/request"
import { useAuthStore } from "@/stores/auth"

vi.mock("axios", async () => {
  const actual = await vi.importActual<typeof import("axios")>("axios")
  const mockAxiosInstance = {
    interceptors: {
      request: { use: vi.fn<() => void>() },
      response: { use: vi.fn<() => void>() },
    },
    get: vi.fn<() => Promise<unknown>>(),
    post: vi.fn<() => Promise<unknown>>(),
    put: vi.fn<() => Promise<unknown>>(),
    patch: vi.fn<() => Promise<unknown>>(),
    delete: vi.fn<() => Promise<unknown>>(),
  }

  return {
    ...actual,
    default: {
      ...actual.default,
      create: vi.fn<() => typeof mockAxiosInstance>(() => mockAxiosInstance),
    },
  }
})

vi.mock("@/router", () => ({
  default: {
    currentRoute: { value: { path: "/dashboard" } },
    push: vi.fn<() => Promise<void>>(),
  },
}))

describe("Request Client", () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it("unwraps successful API response data for GET", async () => {
    const mockData = { id: "trader_1", name: "Alpha Trader" }
    const axiosInstance = axios.create()
    vi.mocked(axiosInstance.get).mockResolvedValue({
      data: {
        success: true,
        data: mockData,
        message: null,
        error: null,
      },
    })

    const result = await request.get("/api/traders/1")
    expect(result).toEqual(mockData)
    expect(axiosInstance.get).toHaveBeenCalledWith("/api/traders/1", undefined)
  })

  it("unwraps successful API response data for POST", async () => {
    const postPayload = { name: "New Trader", balance: 1000 }
    const responseData = { id: "trader_2", ...postPayload }
    const axiosInstance = axios.create()
    vi.mocked(axiosInstance.post).mockResolvedValue({
      data: {
        success: true,
        data: responseData,
        message: "Created",
        error: null,
      },
    })

    const result = await request.post("/api/traders", postPayload)
    expect(result).toEqual(responseData)
    expect(axiosInstance.post).toHaveBeenCalledWith(
      "/api/traders",
      postPayload,
      undefined,
    )
  })

  it("unwraps successful API response data for DELETE", async () => {
    const axiosInstance = axios.create()
    vi.mocked(axiosInstance.delete).mockResolvedValue({
      data: {
        success: true,
        data: { message: "Deleted" },
        message: null,
        error: null,
      },
    })

    const result = await request.delete("/api/traders/1")
    expect(result).toEqual({ message: "Deleted" })
    expect(axiosInstance.delete).toHaveBeenCalledWith(
      "/api/traders/1",
      undefined,
    )
  })

  it("handles request interceptor authorization header injection", () => {
    const authStore = useAuthStore()
    authStore.setToken("my-jwt-token")

    expect(authStore.isLoggedIn).toBe(true)
    expect(authStore.token).toBe("my-jwt-token")
  })
})
