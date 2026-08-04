import type { SettingsResponse, UpdateSettingsRequest } from "@/types/settings"
import request from "@/utils/request"

const Api = {
  Settings: "/api/settings",
} as const

export function getSettingsApi() {
  return request.get<SettingsResponse>(Api.Settings)
}

export function updateSettingsApi(data: UpdateSettingsRequest) {
  return request.put<SettingsResponse>(Api.Settings, data)
}
