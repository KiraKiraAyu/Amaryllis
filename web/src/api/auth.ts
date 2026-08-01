import type {
  AuthStatusPayload,
  MessagePayload,
  SetupConfirmRequest,
  SetupStartPayload,
  TokenPayload,
  VerifyRequest,
} from "@/types/auth"
import request from "@/utils/request"

const Api = {
  Status: "/api/auth/status",
  Verify: "/api/auth/verify",
  SetupStart: "/api/auth/setup/start",
  SetupConfirm: "/api/auth/setup/confirm",
  ResetStart: "/api/auth/reset/start",
  ResetConfirm: "/api/auth/reset/confirm",
} as const

export function getAuthStatusApi() {
  return request.get<AuthStatusPayload>(Api.Status)
}

export function verifyApi(data: VerifyRequest) {
  return request.post<TokenPayload>(Api.Verify, data)
}

export function setupStartApi() {
  return request.post<SetupStartPayload>(Api.SetupStart)
}

export function setupConfirmApi(data: SetupConfirmRequest) {
  return request.post<TokenPayload>(Api.SetupConfirm, data)
}

export function resetStartApi() {
  return request.post<SetupStartPayload>(Api.ResetStart)
}

export function resetConfirmApi(data: SetupConfirmRequest) {
  return request.post<MessagePayload>(Api.ResetConfirm, data)
}
