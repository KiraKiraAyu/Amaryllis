<script setup lang="ts">
import { ref } from "vue"
import Button from "primevue/button"
import InputText from "primevue/inputtext"
import QRCode from "qrcode"
import { resetConfirmApi, resetStartApi } from "@/api/auth"
import { useAuthStore } from "@/stores/auth"
import { useToast } from "@/utils/toast"

const auth = useAuthStore()
const toast = useToast()

const step = ref<"idle" | "scan">("idle")
const secret = ref("")
const qrDataUrl = ref("")
const code = ref("")
const loading = ref(false)
const errorMsg = ref("")

async function startReset() {
  errorMsg.value = ""
  loading.value = true
  try {
    const data = await resetStartApi()
    secret.value = data.secret
    qrDataUrl.value = await QRCode.toDataURL(data.otpauth_url, {
      width: 200,
      margin: 1,
    })
    step.value = "scan"
  } catch (e: unknown) {
    errorMsg.value = e instanceof Error ? e.message : "Failed to start rebind"
  } finally {
    loading.value = false
  }
}

async function confirmReset() {
  errorMsg.value = ""
  if (!/^\d{6}$/.test(code.value.trim())) {
    errorMsg.value = "Enter the 6-digit code from your authenticator app"
    return
  }
  loading.value = true
  try {
    await resetConfirmApi({ code: code.value.trim() })
    toast.add({
      severity: "success",
      summary: "Authenticator Updated",
      detail: "Your authenticator has been rebound successfully",
      life: 3000,
    })
    step.value = "idle"
    secret.value = ""
    qrDataUrl.value = ""
    code.value = ""
  } catch (e: unknown) {
    errorMsg.value = e instanceof Error ? e.message : "Verification failed"
  } finally {
    loading.value = false
  }
}

function cancelReset() {
  step.value = "idle"
  secret.value = ""
  qrDataUrl.value = ""
  code.value = ""
  errorMsg.value = ""
}

function lock() {
  auth.logout()
}
</script>

<template>
  <div class="flex flex-col gap-8 max-w-md mt-4">
    <div>
      <h2 class="font-bold text-lg text-surface-900 dark:text-white mb-2">
        Two-Factor Authentication
      </h2>
      <p class="text-xs mb-4 text-surface-500 font-medium tracking-wide">
        Access is protected by a TOTP authenticator app (Google Authenticator,
        1Password, etc.).
      </p>

      <template v-if="step === 'idle'">
        <Button
          label="Rebind Authenticator"
          icon="pi pi-refresh"
          :loading="loading"
          @click="startReset"
        />
      </template>

      <template v-else>
        <div
          class="flex flex-col gap-4 rounded-xl border border-surface-200 dark:border-surface-700 p-4"
        >
          <p class="text-sm text-surface-600 dark:text-surface-300 font-medium">
            Scan the new QR code with your authenticator app, then enter the
            6-digit code to confirm. The previous secret stops working once this
            is confirmed.
          </p>
          <div v-if="qrDataUrl" class="flex justify-center">
            <img
              :src="qrDataUrl"
              alt="TOTP QR Code"
              class="rounded-lg bg-white p-2 w-[200px] h-[200px]"
            />
          </div>
          <code
            class="block w-full text-center font-mono text-sm bg-surface-100 dark:bg-surface-800 rounded-lg px-3 py-2 select-all text-surface-800 dark:text-surface-100 break-all"
          >
            {{ secret }}
          </code>
          <InputText
            v-model="code"
            inputmode="numeric"
            autocomplete="one-time-code"
            maxlength="6"
            placeholder="123456"
            class="w-full text-center tracking-[0.5em] text-lg font-mono"
          />
          <p
            v-if="errorMsg"
            class="text-xs font-medium text-rose-500 dark:text-rose-400"
          >
            {{ errorMsg }}
          </p>
          <div class="flex gap-3">
            <Button
              label="Confirm Rebind"
              icon="pi pi-check"
              :loading="loading"
              :disabled="loading || code.trim().length !== 6"
              @click="confirmReset"
            />
            <Button
              label="Cancel"
              severity="secondary"
              variant="outlined"
              @click="cancelReset"
            />
          </div>
        </div>
      </template>
    </div>

    <div>
      <h2 class="font-bold text-lg text-surface-900 dark:text-white mb-2">
        Session
      </h2>
      <p class="text-xs mb-4 text-surface-500 font-medium tracking-wide">
        Lock this instance immediately. A valid authenticator code is required
        to unlock.
      </p>
      <Button
        label="Lock"
        icon="pi pi-lock"
        severity="danger"
        variant="outlined"
        @click="lock"
      />
    </div>
  </div>
</template>
