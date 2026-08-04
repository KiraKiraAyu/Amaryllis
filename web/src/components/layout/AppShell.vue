<script setup lang="ts">
import { ref, onMounted, computed, onUnmounted } from "vue"
import { RouterView, RouterLink, useRoute, useRouter } from "vue-router"
import AppSidebar from "@/components/layout/AppSidebar.vue"

const route = useRoute()
const router = useRouter()
const isDark = ref(false)

// --- Route-level carousel transition ---
// Tracks navigation depth to determine slide direction (forward/backward).
// Routes with meta.depth > 0 (e.g. trader-detail) are treated as subpages.
const slideDirection = ref<"forward" | "backward">("forward")

const removeGuard = router.beforeEach((to, from) => {
  const toDepth = (to.meta?.depth as number) ?? 0
  const fromDepth = (from.meta?.depth as number) ?? 0
  slideDirection.value = toDepth >= fromDepth ? "forward" : "backward"
})

onUnmounted(() => removeGuard())

const transitionName = computed(() =>
  slideDirection.value === "backward"
    ? "route-slide-backward"
    : "route-slide-forward",
)

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
      <main class="flex-1 flex flex-col min-h-0 relative overflow-hidden">
        <RouterView v-slot="{ Component, route: currentRoute }">
          <Transition :name="transitionName">
            <div :key="currentRoute.path" class="flex-1 min-h-0 overflow-y-auto py-3 md:py-6">
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
 * Route-level carousel transition (mirrors SlideTransition.vue's effect).
 * Non-scoped so the classes apply to the <Transition> wrapper.
 * Uses a distinct `route-slide-*` prefix to avoid collision with
 * the in-page `carousel-*` classes from SlideTransition.vue.
 *
 * The leaving view is absolutely positioned (inset:0) so the entering
 * view takes its natural place in the flex layout — both slide simultaneously.
 */

.route-slide-forward-enter-active,
.route-slide-forward-leave-active,
.route-slide-backward-enter-active,
.route-slide-backward-leave-active {
  transition: transform 0.4s cubic-bezier(0.4, 0, 0.2, 1);
  will-change: transform;
}

/* Leaving view overlays so entering view takes layout space */
.route-slide-forward-leave-active,
.route-slide-backward-leave-active {
  position: absolute;
  inset: 0;
}

/* Forward: new enters from right, old exits to left */
.route-slide-forward-enter-from {
  transform: translateX(100%);
}
.route-slide-forward-leave-to {
  transform: translateX(-100%);
}

/* Backward: new enters from left, old exits to right */
.route-slide-backward-enter-from {
  transform: translateX(-100%);
}
.route-slide-backward-leave-to {
  transform: translateX(100%);
}
</style>
