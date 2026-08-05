import { onMounted, ref, computed } from "vue"
import { onBeforeRouteLeave } from "vue-router"
import {
  createStrategyApi,
  deleteStrategyApi,
  duplicateStrategyApi,
  getStrategiesApi,
  previewStrategyPromptApi,
  strategyTestRunApi,
  updateStrategyApi,
} from "@/api/strategies"
import type {
  EditableStrategy,
  StrategyPromptPreviewModel,
  StrategyTestResult,
} from "@/types/strategy-ui"

export function useStrategyPage() {
  const strategies = ref<EditableStrategy[]>([])
  const selected = ref<EditableStrategy | null>(null)
  const originalStrategy = ref<EditableStrategy | null>(null)
  const isEditing = ref(false)
  const loading = ref(true)
  const saving = ref(false)
  const testRunLoading = ref(false)
  const testResult = ref<StrategyTestResult | null>(null)
  const previewPromptText = ref<StrategyPromptPreviewModel | null>(null)
  const previewLoading = ref(false)
  const duplicating = ref(false)

  const isDirty = computed(() => {
    if (!isEditing.value || !selected.value || !originalStrategy.value) return false
    return JSON.stringify(selected.value) !== JSON.stringify(originalStrategy.value)
  })

  async function load() {
    loading.value = true
    try {
      const data = await getStrategiesApi()
      strategies.value = data.strategies
    } finally {
      loading.value = false
    }
  }

  function createNew() {
    if (isDirty.value) {
      if (!confirm("You have unsaved changes. Discard them?")) {
        return
      }
    }
    
    selected.value = {
      id: "",
      name: "New Strategy",
      description: "",
      author_email: "",
      is_active: false,
      created_at: "",
      updated_at: "",
      config: {
        symbols: [],
        max_positions: 5,
        prompt_variant: "balanced",
      },
    }
    
    originalStrategy.value = JSON.parse(JSON.stringify(selected.value))
    isEditing.value = true
  }

  function startEdit() {
    if (!selected.value) return
    originalStrategy.value = JSON.parse(JSON.stringify(selected.value))
    isEditing.value = true
  }

  function cancelEdit() {
    if (isDirty.value) {
      if (!confirm("Discard unsaved changes?")) {
        return
      }
    }
    if (!selected.value || !selected.value.id) {
      selected.value = null
    } else if (originalStrategy.value) {
      selected.value = JSON.parse(JSON.stringify(originalStrategy.value))
    }
    isEditing.value = false
    originalStrategy.value = null
  }

  function selectStrategy(strategy: EditableStrategy) {
    if (isDirty.value) {
      if (!confirm("You have unsaved changes. Discard them?")) {
        return
      }
    }
    selected.value = strategy
    isEditing.value = false
    originalStrategy.value = null
    previewPromptText.value = null
    testResult.value = null
  }

  function backToList() {
    if (isDirty.value) {
      if (!confirm("You have unsaved changes. Discard them?")) {
        return
      }
    }
    selected.value = null
    isEditing.value = false
    originalStrategy.value = null
    previewPromptText.value = null
    testResult.value = null
  }

  async function saveStrategy() {
    if (!selected.value) return
    const isNew = !selected.value.id
    saving.value = true
    try {
      if (selected.value.id) {
        await updateStrategyApi(selected.value.id, selected.value)
      } else {
        const data = await createStrategyApi(selected.value)
        selected.value.id = data.id
      }
      await load()
      if (isNew) {
        // After creating a new strategy, return to the list
        selected.value = null
      } else {
        // After editing, show the detail view with refreshed data
        const matched = strategies.value.find(s => s.id === selected.value?.id)
        if (matched) {
          selected.value = matched
        }
      }
      isEditing.value = false
      originalStrategy.value = null
      testResult.value = null
      previewPromptText.value = null
    } finally {
      saving.value = false
    }
  }



  async function deleteStrategy() {
    if (!selected.value?.id) return
    if (!confirm("Delete this strategy?")) return
    await deleteStrategyApi(selected.value.id)
    selected.value = null
    isEditing.value = false
    originalStrategy.value = null
    await load()
  }

  async function runTest() {
    if (!selected.value) return
    testRunLoading.value = true
    testResult.value = null
    try {
      const data = await strategyTestRunApi({
        config: selected.value.config,
        prompt_variant: selected.value.config.prompt_variant ?? "balanced",
        run_real_ai: true,
      })
      testResult.value = data
    } catch (error: unknown) {
      testResult.value = {
        system_prompt: "",
        user_prompt: "",
        prompt_variant: selected.value.config.prompt_variant ?? "balanced",
        ai_model_id: "",
        ai_response:
          error instanceof Error ? error.message : "Strategy test failed",
        decisions: [],
        reasoning: "",
        duration_ms: 0,
        used_real_ai: false,
      }
    } finally {
      testRunLoading.value = false
    }
  }

  async function duplicateStrategy() {
    if (!selected.value?.id) return
    duplicating.value = true
    try {
      const data = await duplicateStrategyApi(selected.value.id, {
        name: `${selected.value.name} Copy`,
      })
      await load()
      selected.value =
        strategies.value.find((strategy) => strategy.id === data.id) ?? null
      isEditing.value = false
      originalStrategy.value = null
    } finally {
      duplicating.value = false
    }
  }

  async function previewPrompt() {
    if (!selected.value) return
    previewLoading.value = true
    try {
      const data = await previewStrategyPromptApi({
        config: selected.value.config,
        prompt_variant: selected.value.config.prompt_variant ?? "balanced",
      })
      previewPromptText.value = {
        system: data.system_prompt,
        variant: data.prompt_variant,
      }
    } finally {
      previewLoading.value = false
    }
  }

  onBeforeRouteLeave((_to, _from, next) => {
    if (isDirty.value) {
      const answer = window.confirm("You have unsaved changes. Do you really want to leave?")
      if (answer) {
        next()
      } else {
        next(false)
      }
    } else {
      next()
    }
  })

  onMounted(load)

  return {
    createNew,
    deleteStrategy,
    duplicateStrategy,
    duplicating,
    loading,
    previewLoading,
    previewPrompt,
    previewPromptText,
    runTest,
    saveStrategy,
    saving,
    selected,
    strategies,
    testResult,
    testRunLoading,
    isEditing,
    isDirty,
    startEdit,
    cancelEdit,
    selectStrategy,
    backToList,
  }
}
