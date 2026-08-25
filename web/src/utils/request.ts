import router from "@/router"
import { useAuthStore } from "@/stores/auth"
import { toast } from "@/utils/toast"
import type { ApiResponse } from "@/types/api"

import axios, {
  AxiosHeaders,
  type AxiosError,
  type AxiosInstance,
  type AxiosRequestConfig,
  type AxiosResponse,
  type InternalAxiosRequestConfig,
} from "axios"

const service: AxiosInstance = axios.create({
  timeout: 10000,
  headers: {
    "Content-Type": "application/json",
  },
})

service.interceptors.request.use(
  (config: InternalAxiosRequestConfig) => {
    const authStore = useAuthStore()

    if (authStore.isLoggedIn && authStore.token) {
      const headers = AxiosHeaders.from(config.headers)
      headers.set("Authorization", `Bearer ${authStore.token}`)
      config.headers = headers
    }

    return config
  },
  (error: AxiosError) => {
    return Promise.reject(error)
  },
)

service.interceptors.response.use(
  (
    response: AxiosResponse<ApiResponse<unknown>>,
  ): AxiosResponse<ApiResponse<unknown>> => {
    const res = response.data

    if (!res.success) {
      const message = res.error || res.message || "请求失败"
      toast.error(message)

      throw new Error(message)
    }

    return response
  },
  async (error: AxiosError<ApiResponse<unknown>>) => {
    const status = error.response?.status

    const message =
      error.response?.data?.error ||
      error.response?.data?.message ||
      error.message ||
      "Network Exception"

    toast.error(message)

    if (status === 401) {
      const authStore = useAuthStore()
      authStore.logout()

      if (router.currentRoute.value.path !== "/login") {
        await router.push("/login")
      }
    }

    return Promise.reject(new Error(message))
  },
)

const request = {
  async get<T = unknown>(url: string, config?: AxiosRequestConfig): Promise<T> {
    const response = await service.get<ApiResponse<T>>(url, config)
    return response.data.data as T
  },

  async post<T = unknown>(
    url: string,
    data?: unknown,
    config?: AxiosRequestConfig,
  ): Promise<T> {
    const response = await service.post<ApiResponse<T>>(url, data, config)
    return response.data.data as T
  },

  async put<T = unknown>(
    url: string,
    data?: unknown,
    config?: AxiosRequestConfig,
  ): Promise<T> {
    const response = await service.put<ApiResponse<T>>(url, data, config)
    return response.data.data as T
  },

  async patch<T = unknown>(
    url: string,
    data?: unknown,
    config?: AxiosRequestConfig,
  ): Promise<T> {
    const response = await service.patch<ApiResponse<T>>(url, data, config)
    return response.data.data as T
  },

  async delete<T = unknown>(
    url: string,
    config?: AxiosRequestConfig,
  ): Promise<T> {
    const response = await service.delete<ApiResponse<T>>(url, config)
    return response.data.data as T
  },
}

export default request
