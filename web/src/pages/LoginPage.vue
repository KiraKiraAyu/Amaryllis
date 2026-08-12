<script setup lang="ts">
import { ref } from "vue"
import { useRouter } from "vue-router"
import InputText from "primevue/inputtext"
import Button from "primevue/button"
import { useAuthStore } from "@/stores/auth"
import { useRealtimeStore } from "@/stores/realtime"

const auth = useAuthStore()
const realtime = useRealtimeStore()
const router = useRouter()

const code = ref("")
const loading = ref(false)
const errorMsg = ref("")

async function submit() {
  errorMsg.value = ""
  if (!/^\d{6}$/.test(code.value.trim())) {
    errorMsg.value = "Enter the 6-digit code from your authenticator app"
    return
  }
  loading.value = true
  try {
    await auth.verify(code.value.trim())
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
  <div
    class="min-h-screen flex items-center justify-center bg-surface-100 dark:bg-surface-950 px-4"
  >
    <div
      class="w-full max-w-sm rounded-2xl bg-surface-50 dark:bg-surface-900 border border-surface-200 dark:border-surface-800 shadow-xl p-8"
    >
      <div class="flex items-center gap-3 mb-8">
        <div
          class="grid h-10 w-10 shrink-0 place-items-center rounded-xl bg-primary-500 text-white font-black text-xl shadow-lg shadow-primary-500/20"
        >
          A
        </div>
        <span
          class="text-xl font-bold tracking-wide text-surface-900 dark:text-surface-0"
          >QuantAura</span
        >
      </div>

      <h1 class="text-lg font-bold text-surface-900 dark:text-white mb-1">
        Two-Factor Authentication
      </h1>
      <p class="text-sm text-surface-500 font-medium mb-6">
        Enter the 6-digit code from your authenticator app to continue.
      </p>

      <form class="flex flex-col gap-4" @submit.prevent="submit">
        <InputText
          v-model="code"
          inputmode="numeric"
          autocomplete="one-time-code"
          maxlength="6"
          placeholder="123456"
          class="w-full text-center tracking-[0.5em] text-lg font-mono"
          autofocus
        />
        <p
          v-if="errorMsg"
          class="text-xs font-medium text-rose-500 dark:text-rose-400"
        >
          {{ errorMsg }}
        </p>
        <Button
          type="submit"
          label="Verify"
          icon="pi pi-shield"
          :loading="loading"
          :disabled="loading || code.trim().length !== 6"
        />
      </form>
    </div>
  </div>
</template>
