<script setup lang="ts">
import { ref, onMounted, computed, onUnmounted } from "vue"
import { RouterView, RouterLink, useRoute, useRouter } from "vue-router"
import AppSidebar from "@/components/layout/AppSidebar.vue"
import { useRealtimeStore } from "@/stores/realtime"

const route = useRoute()
const router = useRouter()
const isDark = ref(false)
const realtime = useRealtimeStore()

// --- Route-level transition direction ---
// First-level routes (both depth 0): vertical slide (up/down by nav order)
// Second-level routes (any depth > 0): horizontal slide (forward/backward)
const NAV_ORDER = [
  "/",
  "/data",
  "/strategy",
  "/backtest",
  "/debate",
  "/traders",
  "/monitor",
  "/settings",
]

function getSectionIndex(path: string): number {
  if (path === "/") return 0
  for (let i = NAV_ORDER.length - 1; i >= 0; i--) {
    const prefix = NAV_ORDER[i]
    if (prefix === "/") continue
    if (path === prefix || path.startsWith(prefix + "/")) return i
  }
  return 0
}

const transitionName = ref("route-v-down")

const removeGuard = router.beforeEach((to, from) => {
  const toDepth = (to.meta?.depth as number) ?? 0
  const fromDepth = (from.meta?.depth as number) ?? 0

  if (toDepth > 0 || fromDepth > 0) {
    // Horizontal: any navigation involving subpages
    transitionName.value =
      toDepth >= fromDepth ? "route-h-forward" : "route-h-backward"
  } else {
    // Vertical: first-level navigation
    const toIdx = getSectionIndex(to.path)
    const fromIdx = getSectionIndex(from.path)
    transitionName.value = toIdx >= fromIdx ? "route-v-down" : "route-v-up"
  }
})

onUnmounted(() => removeGuard())

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
    window.matchMedia &&
    window.matchMedia("(prefers-color-scheme: dark)").matches
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
  <div
    class="h-screen overflow-hidden p-3 md:p-5 pb-22 md:pb-5 transition-colors duration-300 bg-surface-0 dark:bg-surface-950 flex flex-col"
  >
    <AppSidebar :is-dark="isDark" @toggle-theme="toggleDarkMode" />

    <div class="lg:pl-74 flex-1 flex flex-col min-h-0">
      <main class="flex-1 flex flex-col min-h-0 relative overflow-hidden">
        <!-- Offline overlay: covers the route page whenever the SSE
             connection is not established. The wifi icon and "Connection
             lost." text stay fixed; only the Reconnect button transitions
             to show a loading spinner when a reconnect is in progress. -->
        <div
          v-if="!realtime.isConnected && realtime.initialized"
          class="absolute inset-0 z-50 flex flex-col items-center justify-center gap-4 bg-surface-0 dark:bg-surface-950"
        >
          <span
            class="pi pi-wifi text-5xl text-surface-400 dark:text-surface-600"
          ></span>
          <p class="text-lg font-medium text-surface-400 dark:text-surface-600">
            Connection lost.
          </p>
          <button
            class="inline-flex items-center justify-center rounded-xl bg-primary-500 px-6 py-2.5 font-medium text-white shadow-lg shadow-primary-500/20 transition-colors hover:bg-primary-600 disabled:cursor-not-allowed disabled:opacity-80"
            :disabled="realtime.connecting"
            @click="realtime.reconnect()"
          >
            <!-- Spinner: width + opacity + margin transition for smooth fade-in/out -->
            <span
              class="inline-flex items-center justify-center overflow-hidden transition-all duration-300 ease-out"
              :class="
                realtime.connecting
                  ? 'w-5 opacity-100 mr-2'
                  : 'w-0 opacity-0 mr-0'
              "
            >
              <span class="pi pi-spin pi-spinner text-white text-base"></span>
            </span>
            <span>Reconnect</span>
          </button>
        </div>

        <RouterView v-slot="{ Component, route: currentRoute }">
          <Transition :name="transitionName">
            <div
              :key="currentRoute.path"
              class="flex-1 min-h-0 overflow-y-auto py-3 md:py-6"
            >
              <component :is="Component" />
            </div>
          </Transition>
        </RouterView>
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

<style>
/*
 * Route-level transitions.
 * Non-scoped so classes apply to the <Transition> wrapper.
 *
 * Vertical (first-level): new slides in from top or bottom
 * Horizontal (second-level): new slides in from left or right
 *
 * The leaving view is absolutely positioned (inset:0) so both
 * views slide simultaneously — carousel effect.
 */

/* ── Shared ── */
.route-v-down-enter-active,
.route-v-down-leave-active,
.route-v-up-enter-active,
.route-v-up-leave-active,
.route-h-forward-enter-active,
.route-h-forward-leave-active,
.route-h-backward-enter-active,
.route-h-backward-leave-active {
  transition: transform 0.4s cubic-bezier(0.4, 0, 0.2, 1);
  will-change: transform;
}

.route-v-down-leave-active,
.route-v-up-leave-active,
.route-h-forward-leave-active,
.route-h-backward-leave-active {
  position: absolute;
  inset: 0;
}

/* ── Vertical down: new from bottom, old to top ── */
.route-v-down-enter-from {
  transform: translateY(100%);
}
.route-v-down-leave-to {
  transform: translateY(-100%);
}

/* ── Vertical up: new from top, old to bottom ── */
.route-v-up-enter-from {
  transform: translateY(-100%);
}
.route-v-up-leave-to {
  transform: translateY(100%);
}

/* ── Horizontal forward: new from right, old to left ── */
.route-h-forward-enter-from {
  transform: translateX(100%);
}
.route-h-forward-leave-to {
  transform: translateX(-100%);
}

/* ── Horizontal backward: new from left, old to right ── */
.route-h-backward-enter-from {
  transform: translateX(-100%);
}
.route-h-backward-leave-to {
  transform: translateX(100%);
}
</style>
