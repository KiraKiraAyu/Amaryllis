export interface UpdateModelConfigRequest {
  providers?: ProviderConfigInput[]
}

export interface ModelProviderProbeRequest {
  providerType: string
  apiKey?: string
  baseUrl?: string
}

export interface ProviderAvailabilityRequest extends ModelProviderProbeRequest {
  modelId?: string
}

export interface AvailableModelPayload {
  id: string
  name: string
}

export interface AvailableModelListPayload {
  models: AvailableModelPayload[]
}

export interface ProviderAvailabilityPayload {
  available: boolean
  message: string
}

export interface ProviderConfigInput {
  id?: string | null
  name: string
  providerType: string
  enabled: boolean
  apiKey?: string
  baseUrl?: string
  models?: ModelConfigInput[]
}

export interface ModelConfigInput {
  id?: string | null
  name: string
  modelId: string
}

export interface ModelConfigPayload {
  providers: SafeProviderConfig[]
}

export interface SafeProviderConfig {
  id: string
  name: string
  providerType: string
  enabled: boolean
  apiKey: string
  baseUrl: string
  models: SafeModelConfig[]
}

export interface SafeModelConfig {
  id: string
  providerId: string
  name: string
  modelId: string
}

export interface MessagePayload {
  message: string
}
