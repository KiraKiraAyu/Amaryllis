export interface LlmModel {
  id?: string | null
  providerId?: string
  name: string
  modelId: string
}

export interface LlmProvider {
  id?: string | null
  name: string
  providerType: string
  enabled: boolean
  apiKey: string
  baseUrl: string
  models: LlmModel[]
}

export interface ApiCategoryOption {
  value: string
  label: string
}
