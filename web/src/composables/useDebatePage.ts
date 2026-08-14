import { onMounted, ref, watch } from "vue"
import {
  cancelDebateApi,
  createDebateApi,
  getDebateMessagesApi,
  getDebatesApi,
  startDebateApi,
} from "@/api/debates"
import { useRealtimeStore } from "@/stores/realtime"
import type {
  DebateDraft,
  DebateMessage,
  DebateSession,
} from "@/types/debate-ui"

export function useDebatePage() {
  const realtime = useRealtimeStore()
  const personalities = [
    "bull",
    "bear",
    "analyst",
    "contrarian",
    "risk_manager",
  ]
  const debates = ref<DebateSession[]>([])
  const loadingDebates = ref(true)
  const activeDebate = ref<DebateSession | null>(null)
  const messages = ref<DebateMessage[]>([])
  const showCreate = ref(false)
  const creatingDebate = ref(false)
  const newDebate = ref<DebateDraft>({
    name: "",
    symbol: "BTCUSDT",
    max_rounds: 3,
    participants: ["bull", "bear", "analyst"],
  })

  function personalityEmoji(personality: string) {
    const icons: Record<string, string> = {
      bull: "🐂",
      bear: "🐻",
      analyst: "🔬",
      contrarian: "🎭",
      risk_manager: "🛡️",
    }
    return icons[personality] ?? "🤖"
  }

  function togglePersonality(personality: string) {
    const index = newDebate.value.participants.indexOf(personality)
    if (index > -1) newDebate.value.participants.splice(index, 1)
    else newDebate.value.participants.push(personality)
  }

  async function loadDebates() {
    loadingDebates.value = true
    try {
      const data = await getDebatesApi()
      debates.value = data.debates
    } finally {
      loadingDebates.value = false
    }
  }

  async function selectDebate(debate: DebateSession) {
    activeDebate.value = debate
    try {
      const data = await getDebateMessagesApi(debate.id)
      messages.value = data.messages
    } catch {
      messages.value = []
    }
  }

  async function startDebate(id: string) {
    await startDebateApi(id, {})
    await loadDebates()
    if (activeDebate.value?.id === id) {
      activeDebate.value =
        debates.value.find((debate) => debate.id === id) ?? activeDebate.value
    }
  }

  async function cancelDebate(id: string) {
    await cancelDebateApi(id)
    await loadDebates()
  }

  async function createDebate() {
    creatingDebate.value = true
    try {
      await createDebateApi({
        name: newDebate.value.name,
        symbol: newDebate.value.symbol.toUpperCase(),
        max_rounds: newDebate.value.max_rounds,
        participants: newDebate.value.participants,
      })
      showCreate.value = false
      await loadDebates()
    } finally {
      creatingDebate.value = false
    }
  }

  watch(
    () => realtime.lastEvent,
    (event) => {
      if (!event || !activeDebate.value) return
      if (
        event.type === "debate_message" &&
        (event.debate_id as string) === activeDebate.value.id
      ) {
        messages.value.push({
          id: String(event.id ?? ""),
          round: Number(event.round ?? 0),
          personality: String(event.personality ?? ""),
          role: String(event.role ?? ""),
          content: String(event.content ?? ""),
          vote: String(event.vote ?? ""),
          created_at: Number(event.created_at ?? Math.floor(Date.now() / 1000)),
        })
      }
      if (
        event.type === "debate_finished" &&
        (event.debate_id as string) === activeDebate.value.id
      ) {
        activeDebate.value.status = "completed"
        activeDebate.value.final_decision = event.final_decision as string
        activeDebate.value.final_reasoning = event.final_reasoning as string
        void loadDebates()
      }
    },
  )

  onMounted(loadDebates)

  return {
    activeDebate,
    cancelDebate,
    createDebate,
    creatingDebate,
    debates,
    loadingDebates,
    messages,
    newDebate,
    personalities,
    personalityEmoji,
    selectDebate,
    showCreate,
    startDebate,
    togglePersonality,
  }
}
