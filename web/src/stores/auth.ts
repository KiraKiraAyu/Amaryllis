import { defineStore } from "pinia"
import { computed, ref } from "vue"
import { getAuthStatusApi, verifyApi } from "@/api/auth"
import router from "@/router"

const TOKEN_KEY = "quantaura_token"

export const useAuthStore = defineStore("auth", () => {
  const token = ref<string>(localStorage.getItem(TOKEN_KEY) ?? "")
  // null = not probed yet; true/false = server-side TOTP configuration state
  const configured = ref<boolean | null>(null)

  const isLoggedIn = computed(() => !!token.value)

  function setToken(tok: string) {
    token.value = tok
    localStorage.setItem(TOKEN_KEY, tok)
  }

  async function refreshStatus() {
    const status = await getAuthStatusApi()
    configured.value = status.configured
    return status.configured
  }

  async function verify(code: string) {
    const data = await verifyApi({ code })
    setToken(data.token)
    return data
  }

  function logout() {
    token.value = ""
    localStorage.removeItem(TOKEN_KEY)
    router.push("/login")
  }

  return {
    token,
    configured,
    isLoggedIn,
    setToken,
    refreshStatus,
    verify,
    logout,
  }
})
