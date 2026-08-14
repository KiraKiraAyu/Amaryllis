import { onMounted, onUnmounted, ref, computed, watch } from "vue"
import { onBeforeRouteLeave } from "vue-router"
import {
  createStrategyApi,
  deleteStrategyApi,
  duplicateStrategyApi,
  getDefaultStrategyConfigApi,
  getStrategiesApi,
  getStrategyApi,
  getStrategyPositionsApi,
  previewStrategyPromptApi,
  strategyTestRunApi,
  updateStrategyApi,
} from "@/api/strategies"
import type {
  EditableStrategy,
  StrategyPromptPreviewModel,
  StrategyTestResult,
} from "@/types/strategy-ui"
import type {
  StrategyConfigPayload,
  StrategyTpSlConfigPayload,
} from "@/types/strategies"
import type { PositionPayload } from "@/types/trading"
import { ensureDataTemplate } from "@/components/strategy/editor/data-template"

const POSITIONS_POLL_MS = 5000

/**
 * Ensures all config sub-objects required by the UI exist.
 * Called after loading strategies from the server and when creating a new strategy.
 * This allows components to safely read config fields without side-effecting computeds.
 */
function ensureStrategyConfig(config: StrategyConfigPayload): void {
  if (!config.symbols) config.symbols = []
  ensureDataTemplate(config)
  if (!config.tp_sl) {
    const empty: StrategyTpSlConfigPayload = {
      take_profit: { mode: "fixed", pnl_rate: null, custom_prompt: null },
      stop_loss: { mode: "fixed", pnl_rate: null, custom_prompt: null },
    }
    config.tp_sl = empty
  }
  if (config.max_positions == null) config.max_positions = 5
}

function errorResult(
  message: string,
): StrategyTestResult {
  return {
    system_prompt: "",
    user_prompt: "",
    ai_model_id: "",
    ai_response: message,
    decisions: [],
    reasoning: "",
    duration_ms: 0,
    used_real_ai: false,
  }
}

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
  const positions = ref<PositionPayload[]>([])
  const positionsLoading = ref(false)
  let positionsTimer: ReturnType<typeof setInterval> | null = null
  let positionsRequestId = 0

  // Cached backend default config — fetched once on mount, used by createNew.
  const defaultConfig = ref<StrategyConfigPayload | null>(null)

  const isDirty = computed(() => {
    if (!isEditing.value || !selected.value || !originalStrategy.value)
      return false
    return (
      JSON.stringify(selected.value) !== JSON.stringify(originalStrategy.value)
    )
  })

  async function load() {
    loading.value = true
    try {
      const data = await getStrategiesApi()
      // Normalize configs so UI components can safely read all fields
      for (const s of data.strategies) {
        if (s.config) ensureStrategyConfig(s.config)
      }
      strategies.value = data.strategies
    } finally {
      loading.value = false
    }
  }

  async function loadPositions() {
    if (!selected.value?.id) {
      positions.value = []
      return
    }
    // Track the latest request to discard stale responses on rapid strategy switches
    const requestId = ++positionsRequestId
    positionsLoading.value = true
    try {
      const data = await getStrategyPositionsApi(selected.value.id)
      if (requestId !== positionsRequestId) return // stale
      positions.value = data.items
    } catch {
      // silently ignore — positions are best-effort
    } finally {
      if (requestId === positionsRequestId) {
        positionsLoading.value = false
      }
    }
  }

  function startPositionsPolling() {
    stopPositionsPolling()
    loadPositions()
    positionsTimer = setInterval(loadPositions, POSITIONS_POLL_MS)
  }

  function stopPositionsPolling() {
    if (positionsTimer) {
      clearInterval(positionsTimer)
      positionsTimer = null
    }
    positions.value = []
  }

  // Auto-manage polling based on selection and editing state
  watch(
    () => [selected.value?.id, isEditing.value] as const,
    ([id, editing]) => {
      if (id && !editing) {
        startPositionsPolling()
      } else {
        stopPositionsPolling()
      }
    },
  )

  function createNew() {
    if (isDirty.value) {
      if (!confirm("You have unsaved changes. Discard them?")) {
        return
      }
    }

    // Use the backend default config (richer than any frontend fallback).
    // Fall back to a minimal config if the fetch hasn't completed yet.
    const config: StrategyConfigPayload = defaultConfig.value
      ? JSON.parse(JSON.stringify(defaultConfig.value))
      : {
          symbols: [],
          max_positions: 5,
        }
    ensureStrategyConfig(config)

    selected.value = {
      id: "",
      name: "New Strategy",
      description: "",
      author_email: "",
      is_active: false,
      created_at: "",
      updated_at: "",
      config,
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
        await createStrategyApi(selected.value)
      }
      await load()
      if (isNew) {
        // After creating a new strategy, return to the list
        selected.value = null
      } else {
        // After editing, show the detail view with refreshed data
        const matched = strategies.value.find(
          (s) => s.id === selected.value?.id,
        )
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
        run_real_ai: true,
      })
      testResult.value = data
    } catch (error: unknown) {
      const errorMsg =
        error instanceof Error ? error.message : "Strategy test failed"
      // If real AI fails (e.g. no API key configured), fall back to simulated mode
      const isApiKeyError = errorMsg.toLowerCase().includes("api key")
      if (isApiKeyError) {
        try {
          const data = await strategyTestRunApi({
            config: selected.value.config,
            run_real_ai: false,
          })
          testResult.value = data
        } catch {
          testResult.value = errorResult(errorMsg)
        }
      } else {
        testResult.value = errorResult(errorMsg)
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
      // Find the duplicated strategy in the refreshed list.
      // Fall back to a direct fetch if the list doesn't contain it yet.
      const found =
        strategies.value.find((s) => s.id === data.id) ?? null
      if (found) {
        selected.value = found
      } else {
        try {
          const fetched = await getStrategyApi(data.id)
          if (fetched.config) ensureStrategyConfig(fetched.config)
          selected.value = fetched
        } catch {
          selected.value = null
        }
      }
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
      })
      previewPromptText.value = {
        system: data.system_prompt,
      }
    } finally {
      previewLoading.value = false
    }
  }

  onBeforeRouteLeave((_to, _from, next) => {
    if (isDirty.value) {
      const answer = window.confirm(
        "You have unsaved changes. Do you really want to leave?",
      )
      if (answer) {
        next()
      } else {
        next(false)
      }
    } else {
      next()
    }
  })

  onMounted(() => {
    load()
    // Pre-fetch backend default config for createNew (best-effort, non-blocking)
    getDefaultStrategyConfigApi()
      .then((data) => {
        defaultConfig.value = data.config
      })
      .catch(() => {
        // Non-critical — createNew will fall back to a minimal config
      })
  })

  onUnmounted(() => {
    stopPositionsPolling()
  })

  return {
    createNew,
    deleteStrategy,
    duplicateStrategy,
    duplicating,
    loading,
    positions,
    positionsLoading,
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
