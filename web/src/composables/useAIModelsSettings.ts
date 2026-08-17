import { computed, onMounted, ref } from "vue"
import { getSupportedProviderTypesApi } from "@/api/catalog"
import {
  checkProviderAvailabilityApi,
  getModelConfigsApi,
  listAvailableModelsApi,
  updateModelConfigsApi,
} from "@/api/models"
import type {
  AvailableModelPayload,
  UpdateModelConfigRequest,
} from "@/types/models"
import type { SupportedProviderTypePayload } from "@/types/public"
import type { LlmModel, LlmProvider } from "@/types/ai-models-ui"

export function useAIModelsSettings() {
  const supportedProviderTypes = ref<SupportedProviderTypePayload[]>([])
  const providers = ref<LlmProvider[]>([])
  const selectedProviderIndex = ref(0)
  const savingModels = ref(false)
  const checkingProvider = ref(false)
  const fetchingRemoteModels = ref(false)
  const modelDialogOpen = ref(false)
  const modelDialogIsNew = ref(true)
  const modelDraft = ref(createModel())
  const editingModelIndex = ref(-1)
  const checkMessage = ref("")
  const remoteModels = ref<AvailableModelPayload[]>([])
  const providerCheckModalOpen = ref(false)
  const providerCheckModelId = ref("")

  const apiCategories = computed(() => {
    return supportedProviderTypes.value.map((providerType) => ({
      value: providerType.providerType,
      label: providerType.name || apiCategoryName(providerType.providerType),
    }))
  })

  const activeProvider = computed(
    () => providers.value[selectedProviderIndex.value],
  )

  function createModel(): LlmModel {
    return {
      name: "",
      modelId: "",
    }
  }

  function providerProbePayload(provider: LlmProvider) {
    return {
      providerType: provider.providerType,
      apiKey: provider.apiKey,
      baseUrl: provider.baseUrl,
    }
  }

  function createProvider(
    providerType = supportedProviderTypes.value[0],
  ): LlmProvider {
    return {
      name: providerType?.name ?? "",
      providerType: providerType?.providerType ?? "",
      enabled: true,
      apiKey: "",
      baseUrl: "",
      models: [],
    }
  }

  function normalizeProvider(raw: Partial<LlmProvider>): LlmProvider {
    return {
      id: raw.id,
      name: raw.name ?? "",
      providerType: raw.providerType ?? "",
      enabled: raw.enabled ?? true,
      apiKey: raw.apiKey ?? "",
      baseUrl: raw.baseUrl ?? "",
      models: Array.isArray(raw.models)
        ? raw.models.map((model) => ({
            id: model.id,
            providerId: model.providerId,
            name: model.name ?? "",
            modelId: model.modelId ?? "",
          }))
        : [],
    }
  }

  function providerKey(provider: LlmProvider, index: number) {
    return provider.id ?? `provider-${index}`
  }

  function providerLabel(provider: LlmProvider) {
    return provider.name || provider.providerType || "Untitled Provider"
  }

  function apiCategoryLabel(providerType: string) {
    return (
      apiCategories.value.find((category) => category.value === providerType)
        ?.label ?? providerType
    )
  }

  function apiCategoryName(value: string) {
    const normalized = value.trim().toLowerCase()
    if (normalized === "chat_completions") return "Chat Completions"
    if (normalized === "responses") return "Responses"
    if (normalized === "anthropic_messages") return "Anthropic Messages"
    if (normalized === "gemini_generate_content") return "Gemini GenerateContent"
    return value
  }

  async function loadModels() {
    try {
      const data = await getModelConfigsApi()
      providers.value = Array.isArray(data?.providers)
        ? data.providers.map((provider) => normalizeProvider(provider))
        : []
      selectedProviderIndex.value = providers.value.length > 0 ? 0 : -1
    } catch {
      providers.value = []
      selectedProviderIndex.value = -1
    }
  }

  async function loadProviderCatalog() {
    try {
      supportedProviderTypes.value = await getSupportedProviderTypesApi()
    } catch {
      supportedProviderTypes.value = []
    }
  }

  async function saveModels() {
    savingModels.value = true
    const selectedId = activeProvider.value?.id
    const selectedIndex = selectedProviderIndex.value

    try {
      const payload: UpdateModelConfigRequest = {
        providers: providers.value.map((provider) => ({
          id: provider.id,
          name: provider.name,
          providerType: provider.providerType,
          enabled: provider.enabled,
          apiKey: provider.apiKey,
          baseUrl: provider.baseUrl,
          models: provider.models
            .filter((model) => model.name.trim() || model.modelId.trim())
            .map((model) => ({
              id: model.id,
              name: model.name,
              modelId: model.modelId,
            })),
        })),
      }

      await updateModelConfigsApi(payload)
      await loadModels()

      if (providers.value.length === 0) {
        selectedProviderIndex.value = -1
        return
      }

      const nextSelectedIndex = selectedId
        ? providers.value.findIndex((provider) => provider.id === selectedId)
        : Math.min(selectedIndex, providers.value.length - 1)
      selectedProviderIndex.value = Math.max(nextSelectedIndex, 0)
    } finally {
      savingModels.value = false
    }
  }

  function selectProvider(index: number) {
    selectedProviderIndex.value = index
    modelDialogOpen.value = false
    checkMessage.value = ""
    remoteModels.value = []
    providerCheckModalOpen.value = false
    providerCheckModelId.value = ""
  }

  function addProvider() {
    if (supportedProviderTypes.value.length === 0) return
    providers.value.push(createProvider())
    selectProvider(providers.value.length - 1)
  }

  function removeProvider(index: number) {
    if (!confirm("Delete this provider and its models?")) return
    providers.value.splice(index, 1)
    modelDialogOpen.value = false
    selectedProviderIndex.value = Math.min(
      selectedProviderIndex.value,
      providers.value.length - 1,
    )
  }

  function openProviderCheckModal(provider: LlmProvider) {
    checkMessage.value = ""
    providerCheckModelId.value =
      provider.models.find((model) => model.modelId.trim())?.modelId ?? ""
    providerCheckModalOpen.value = true
  }

  function closeProviderCheckModal() {
    providerCheckModalOpen.value = false
  }

  async function checkProvider(provider: LlmProvider | undefined) {
    if (!provider || !providerCheckModelId.value.trim()) return

    checkingProvider.value = true
    checkMessage.value = ""

    try {
      await checkProviderAvailabilityApi({
        ...providerProbePayload(provider),
        modelId: providerCheckModelId.value,
      })
      checkMessage.value = `Provider test successful with ${providerCheckModelId.value}`
      providerCheckModalOpen.value = false
    } finally {
      checkingProvider.value = false
    }
  }

  async function fetchRemoteModels(provider: LlmProvider) {
    fetchingRemoteModels.value = true
    remoteModels.value = []

    try {
      const payload = await listAvailableModelsApi(
        providerProbePayload(provider),
      )
      remoteModels.value = payload.models
    } finally {
      fetchingRemoteModels.value = false
    }
  }

  function hasModel(provider: LlmProvider, modelId: string) {
    return provider.models.some((model) => model.modelId === modelId)
  }

  function addRemoteModel(provider: LlmProvider, model: AvailableModelPayload) {
    if (hasModel(provider, model.id)) return
    provider.models.push({
      name: model.name || model.id,
      modelId: model.id,
    })
  }

  function startAddModel() {
    modelDraft.value = createModel()
    modelDialogIsNew.value = true
    editingModelIndex.value = -1
    modelDialogOpen.value = true
  }

  function startEditModel(provider: LlmProvider, modelIndex: number) {
    const model = provider.models[modelIndex]
    if (!model) return
    modelDraft.value = { ...model }
    modelDialogIsNew.value = false
    editingModelIndex.value = modelIndex
    modelDialogOpen.value = true
  }

  function saveModelDraft(provider: LlmProvider) {
    if (!modelDraft.value.name.trim() || !modelDraft.value.modelId.trim()) {
      return
    }
    if (modelDialogIsNew.value) {
      provider.models.push({ ...modelDraft.value })
    } else if (editingModelIndex.value >= 0) {
      provider.models.splice(editingModelIndex.value, 1, {
        ...modelDraft.value,
      })
    }
    modelDialogOpen.value = false
  }

  function removeModel(provider: LlmProvider, modelIndex: number) {
    provider.models.splice(modelIndex, 1)
  }

  onMounted(async () => {
    await Promise.all([loadProviderCatalog(), loadModels()])
  })

  return {
    activeProvider,
    addProvider,
    addRemoteModel,
    apiCategories,
    apiCategoryLabel,
    checkMessage,
    checkingProvider,
    closeProviderCheckModal,
    fetchRemoteModels,
    fetchingRemoteModels,
    hasModel,
    modelDialogOpen,
    modelDialogIsNew,
    modelDraft,
    startEditModel,
    saveModelDraft,
    openProviderCheckModal,
    providers,
    providerCheckModalOpen,
    providerCheckModelId,
    providerKey,
    providerLabel,
    remoteModels,
    removeModel,
    removeProvider,
    saveModels,
    savingModels,
    selectProvider,
    selectedProviderIndex,
    startAddModel,
    supportedProviderTypes,
    checkProvider,
  }
}
