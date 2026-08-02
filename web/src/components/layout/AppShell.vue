<script setup lang="ts">
import { ref, onMounted, computed } from "vue"
import { RouterView, RouterLink, useRoute } from "vue-router"
import AppSidebar from "@/components/layout/AppSidebar.vue"

const route = useRoute()
const isDark = ref(false)

const nav = computed(() => [
  { label: "Dashboard", to: "/", icon: "pi pi-chart-bar" },
  { label: "Market Data", to: "/data", icon: "pi pi-chart-line" },
  { label: "Strategy", to: "/strategy", icon: "pi pi-sliders-h" },
  { label: "Backtest", to: "/backtest", icon: "pi pi-history" },
  { label: "AI Debate", to: "/debate", icon: "pi pi-comments" },
  { label: "Traders", to: "/traders", icon: "pi pi-users" },
  { label: "Monitor", to: "/monitor", icon: "pi pi-server" },
  { label: "Settings", to: "/settings", icon: "pi pi-cog" },
])

function toggleDarkMode() {
  isDark.value = !isDark.value
  if (isDark.value) {
    document.documentElement.classList.add("dark")
    localStorage.setItem("quantaura.theme", "dark")
  } else {
    document.documentElement.classList.remove("dark")
    localStorage.setItem("quantaura.theme", "light")
  }
}

onMounted(() => {
  const savedTheme = localStorage.getItem("quantaura.theme")
  const prefersDark =
    window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches
  if (savedTheme === "dark" || (!savedTheme && prefersDark)) {
    isDark.value = true
    document.documentElement.classList.add("dark")
  } else {
    isDark.value = false
    document.documentElement.classList.remove("dark")
  }
})
</script>

<template>
  <!-- Bottom nav is 4.5rem tall on mobile, pb-22 avoids overlaps -->
  <div class="h-screen overflow-hidden p-3 md:p-5 pb-22 md:pb-5 transition-colors duration-300 bg-surface-0 dark:bg-surface-950 flex flex-col">
    <AppSidebar :is-dark="isDark" @toggle-theme="toggleDarkMode" />

    <div class="lg:pl-74 flex-1 flex flex-col min-h-0">
      <main class="py-3 md:py-6 flex-1 flex flex-col min-h-0 overflow-y-auto">
        <RouterView />
      </main>
    </div>

    <!-- Floating Bottom Navigation Bar for Mobile & Tablet (hidden on lg screens) -->
    <nav
      class="fixed bottom-3 inset-x-3 z-30 lg:hidden bg-surface-0/90 dark:bg-surface-900/90 backdrop-blur-md border border-surface-200 dark:border-surface-800 rounded-2xl flex justify-around py-2 px-4 shadow-xl safe-bottom transition-all duration-300"
    >
      <RouterLink
        v-for="item in nav"
        :key="item.to"
        :to="item.to"
        class="flex flex-col items-center gap-1.5 text-xs font-semibold text-surface-500 dark:text-surface-400 py-1 transition-all"
        :class="{ 'text-primary scale-105 font-bold': route.path === item.to }"
      >
        <span :class="item.icon" class="text-base"></span>
        <span class="scale-90">{{ item.label }}</span>
      </RouterLink>
    </nav>
  </div>
</template>
