export interface CreateDebateRequest {
  name?: string | null
  symbol?: string | null
  max_rounds?: number | null
  participants?: string[] | null
}

export interface StartDebateRequest {
  model_ids?: string[] | null
}

export interface DebateSummaryPayload {
  id: string
  name: string
  symbol: string
  status: string
  max_rounds: number
  current_round: number
  final_decision: string
  created_at: number
}

export interface DebatePayload extends DebateSummaryPayload {
  participants: string[]
  final_reasoning: string
  error_message: string
  updated_at: number
}

export interface DebateRoundVotePayload {
  personality: string
  vote: string
}

export interface DebateVoteRoundPayload {
  round: number
  votes: DebateRoundVotePayload[]
  tally: Record<string, number>
}

export interface DebateListPayload {
  debates: DebateSummaryPayload[]
  count: number
}

export interface DebateDetailPayload {
  debate: DebatePayload
}

export interface DebateActionPayload {
  id: string
  message: string
}

export interface DebateMessagePayload {
  message: string
}

export interface DebatePersonalityPayload {
  id: string
  name: string
  description: string
}

export interface DebatePersonalitiesPayload {
  personalities: DebatePersonalityPayload[]
}

export interface DebateExecutionPayload {
  id: string
  final_decision: string
  final_reasoning: string
  message: string
}

export interface DebateMessagesPayload {
  messages: DebateMessageItemPayload[]
  count: number
}

export interface DebateMessageItemPayload {
  id: string
  round: number
  personality: string
  role: string
  content: string
  vote: string
  created_at: number
}

export interface DebateVotesPayload {
  votes: DebateVoteRoundPayload[]
}
