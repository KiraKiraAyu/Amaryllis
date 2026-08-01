export interface VerifyRequest {
  code: string
}

export interface SetupConfirmRequest {
  code: string
}

export interface AuthStatusPayload {
  configured: boolean
}

export interface SetupStartPayload {
  secret: string
  otpauth_url: string
}

export interface TokenPayload {
  token: string
  message: string
}

export interface MessagePayload {
  message: string
}
