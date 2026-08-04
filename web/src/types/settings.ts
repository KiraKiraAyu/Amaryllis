export interface SettingsResponse {
  timezone: string | null
}

export interface UpdateSettingsRequest {
  timezone?: string | null
}
