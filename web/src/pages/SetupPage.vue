<script setup lang="ts">
import { onMounted, ref } from "vue"
import { useRouter } from "vue-router"
import InputText from "primevue/inputtext"
import Button from "primevue/button"
import QRCode from "qrcode"
import { setupConfirmApi, setupStartApi } from "@/api/auth"
import { useAuthStore } from "@/stores/auth"
import { useRealtimeStore } from "@/stores/realtime"

const auth = useAuthStore()
const realtime = useRealtimeStore()
const router = useRouter()

const secret = ref("")
const qrDataUrl = ref("")
const code = ref("")
const loading = ref(false)
const initializing = ref(true)
const errorMsg = ref("")

onMounted(async () => {
  try {
    const data = await setupStartApi()
    secret.value = data.secret
    qrDataUrl.value = await QRCode.toDataURL(data.otpauth_url, {
      width: 220,
      margin: 1,
    })
  } catch (e: unknown) {
    errorMsg.value = e instanceof Error ? e.message : "Failed to start setup"
  } finally {
    initializing.value = false
  }
})

async function submit() {
  errorMsg.value = ""
  if (!/^\d{6}$/.test(code.value.trim())) {
    errorMsg.value = "Enter the 6-digit code from your authenticator app"
    return
  }
  loading.value = true
  try {
    const data = await setupConfirmApi({ code: code.value.trim() })
    auth.setToken(data.token)
    auth.configured = true
    realtime.connect()
    await router.push({ name: "dashboard" })
  } catch (e: unknown) {
    errorMsg.value = e instanceof Error ? e.message : "Verification failed"
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="min-h-screen flex items-center justify-center bg-surface-100 dark:bg-surface-950 px-4 py-10">
    <div
      class="w-full max-w-md rounded-2xl bg-surface-50 dark:bg-surface-900 border border-surface-200 dark:border-surface-800 shadow-xl p-8"
    >
      <div class="flex items-center gap-3 mb-6">
        <div
          class="grid h-10 w-10 shrink-0 place-items-center rounded-xl bg-primary-500 text-white font-black text-xl shadow-lg shadow-primary-500/20"
        >
          A
        </div>
        <span class="text-xl font-bold tracking-wide text-surface-900 dark:text-surface-0">QuantAura</span>
      </div>

      <h1 class="text-lg font-bold text-surface-900 dark:text-white mb-1">Set Up Two-Factor Authentication</h1>
      <p class="text-sm text-surface-500 font-medium mb-6">
        Scan the QR code with an authenticator app (Google Authenticator, 1Password, etc.),
        then enter the 6-digit code to finish setup.
      </p>

      <div v-if="initializing" class="flex justify-center py-10">
        <i class="pi pi-spin pi-spinner text-3xl text-primary-500"></i>
      </div>

      <template v-else>
        <div v-if="qrDataUrl" class="flex justify-center mb-4">
          <img :src="qrDataUrl" alt="TOTP QR Code" class="rounded-xl bg-white p-3 w-[220px] h-[220px]" />
        </div>

        <div class="mb-6">
          <label class="block text-xs font-semibold text-surface-500 uppercase tracking-wide mb-1">
            Manual entry key
          </label>
          <code
            class="block w-full text-center font-mono text-sm bg-surface-100 dark:bg-surface-800 rounded-lg px-3 py-2 select-all text-surface-800 dark:text-surface-100 break-all"
          >
            {{ secret }}
          </code>
        </div>

        <form class="flex flex-col gap-4" @submit.prevent="submit">
          <InputText
            v-model="code"
            inputmode="numeric"
            autocomplete="one-time-code"
            maxlength="6"
            placeholder="123456"
            class="w-full text-center tracking-[0.5em] text-lg font-mono"
          />
          <p v-if="errorMsg" class="text-xs font-medium text-rose-500 dark:text-rose-400">
            {{ errorMsg }}
          </p>
          <Button
            type="submit"
            label="Confirm & Enable"
            icon="pi pi-check"
            :loading="loading"
            :disabled="loading || code.trim().length !== 6"
          />
        </form>
      </template>
    </div>
  </div>
</template>
