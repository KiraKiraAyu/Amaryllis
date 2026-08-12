<script setup lang="ts">
import { RouterView } from "vue-router"
import { onMounted, watch } from "vue"
import { useAuthStore } from "@/stores/auth"
import { useRealtimeStore } from "@/stores/realtime"
import { useSettingsStore } from "@/stores/settings"
import Toast from "primevue/toast"
import { useToast } from "@/stores/toast"
import { useToast as usePrimeToast } from "primevue/usetoast"

const auth = useAuthStore()
const realtime = useRealtimeStore()
const settings = useSettingsStore()
const toastStore = useToast()
const primeToast = usePrimeToast()

// Watch custom Pinia toast store to dispatch toast messages via PrimeVue Toast globally
watch(
  () => toastStore.toastEvent,
  (evt) => {
    if (evt) {
      primeToast.add({
        severity: evt.severity,
        summary: evt.summary,
        detail: evt.detail,
        life: evt.life,
      })
      toastStore.toastEvent = null // Reset channel
    }
  },
  { deep: true },
)

onMounted(() => {
  if (auth.isLoggedIn) {
    realtime.connect()
    void settings.load()
  }

  // Theme initialization
  const savedTheme = localStorage.getItem("quantaura.theme")
  const prefersDark =
    window.matchMedia &&
    window.matchMedia("(prefers-color-scheme: dark)").matches
  if (savedTheme === "dark" || (!savedTheme && prefersDark)) {
    document.documentElement.classList.add("dark")
  } else {
    document.documentElement.classList.remove("dark")
  }
})

watch(
  () => auth.token,
  (token) => {
    if (token) {
      realtime.connect()
      if (!settings.initialized) {
        void settings.load()
      }
    } else {
      realtime.disconnect()
    }
  },
)
</script>

<template>
  <Toast />
  <RouterView />
</template>
