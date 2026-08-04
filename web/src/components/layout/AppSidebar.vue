<script setup lang="ts">
import { computed, ref, watch, nextTick, onMounted } from "vue"
import { useRoute, RouterLink } from "vue-router"
import { useAuthStore } from "@/stores/auth"

defineProps<{ isDark: boolean }>()
const emit = defineEmits<{ (e: "toggle-theme"): void }>()

const route = useRoute()
const authStore = useAuthStore()

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

function isActive(path: string): boolean {
  if (path === "/") return route.path === "/"
  return route.path === path || route.path.startsWith(path + "/")
}

const activeIndex = computed(() => {
  for (let i = 0; i < nav.value.length; i++) {
    if (isActive(nav.value[i].to)) return i
  }
  return 0
})

// --- Sliding active indicator ---
const linkEls = ref<HTMLElement[]>([])
const indicatorTop = ref(0)
const indicatorHeight = ref(0)
const indicatorReady = ref(false)

function setLinkRef(el: unknown, i: number) {
  if (!el) return
  const dom = (el as { $el?: HTMLElement })?.$el ?? (el as HTMLElement)
  if (dom instanceof HTMLElement) {
    linkEls.value[i] = dom
  }
}

function updateIndicator() {
  const el = linkEls.value[activeIndex.value]
  if (el) {
    indicatorTop.value = el.offsetTop
    indicatorHeight.value = el.offsetHeight
  }
}

watch(activeIndex, async () => {
  await nextTick()
  updateIndicator()
})

onMounted(() => {
  nextTick(() => {
    updateIndicator()
    requestAnimationFrame(() => {
      indicatorReady.value = true
    })
  })
})
</script>

<template>
  <aside
    class="fixed inset-y-5 left-5 hidden w-64 flex-col items-stretch rounded-2xl bg-surface-50 dark:bg-surface-900 px-4 py-6 lg:flex transition-colors duration-300"
  >
    <!-- Header -->
    <div class="flex items-center gap-3 px-2">
      <div
        class="grid h-10 w-10 shrink-0 place-items-center rounded-xl bg-primary-500 text-white font-black text-xl shadow-lg shadow-primary-500/20"
        aria-label="QuantAura"
      >
        A
      </div>
      <span class="text-xl font-bold tracking-wide text-surface-900 dark:text-surface-0">QuantAura</span>
    </div>

    <!-- Nav Items -->
    <nav class="mt-8 grid gap-2 relative sidebar-nav">
      <!-- Sliding active indicator -->
      <div
        class="nav-indicator"
        :class="{ 'nav-indicator-animated': indicatorReady }"
        :style="{ top: `${indicatorTop}px`, height: `${indicatorHeight}px` }"
      ></div>

      <RouterLink
        v-for="(item, i) in nav"
        :key="item.to"
        :ref="(el) => setLinkRef(el, i)"
        :to="item.to"
        class="nav-link"
        :class="{ 'is-active': isActive(item.to) }"
        :aria-label="item.label"
        :title="item.label"
      >
        <span :class="item.icon" class="text-lg w-6 text-center"></span>
        <span>{{ item.label }}</span>
      </RouterLink>
    </nav>

    <!-- Bottom Actions -->
    <div class="mt-auto flex flex-col gap-3">
      <!-- Dark Mode Switcher -->
      <button
        class="cursor-pointer h-11 w-full px-4 flex items-center gap-3 rounded-xl text-surface-600 dark:text-surface-400 hover:bg-surface-100 dark:hover:bg-surface-800 hover:text-surface-900 dark:hover:text-surface-0 transition-colors font-medium text-sm"
        @click="emit('toggle-theme')"
      >
        <span class="pi text-lg w-6 text-center" :class="isDark ? 'pi-sun' : 'pi-moon'"></span>
        <span>{{ isDark ? 'Light Mode' : 'Dark Mode' }}</span>
      </button>

      <!-- Lock Session -->
      <button
        class="cursor-pointer h-11 w-full px-4 flex items-center gap-3 rounded-xl text-surface-600 dark:text-surface-400 hover:bg-surface-100 dark:hover:bg-surface-800 hover:text-surface-900 dark:hover:text-surface-0 transition-colors font-medium text-sm border-t border-surface-200 dark:border-surface-800 pt-4 mt-1"
        title="Lock"
        @click="authStore.logout()"
      >
        <span class="pi pi-lock text-lg w-6 text-center"></span>
        <span>Lock</span>
      </button>
    </div>
  </aside>
</template>

<style>
/*
 * Sidebar sliding active indicator.
 * Non-scoped because .nav-link is defined globally in style.css.
 * Scoped to .sidebar-nav to avoid affecting the mobile bottom nav.
 */

.sidebar-nav {
  position: relative;
}

.sidebar-nav .nav-link {
  position: relative;
  z-index: 1;
  transition: background-color 0.3s ease, color 0.3s ease;
}

/* Override .is-active background — it's handled by the sliding indicator */
.sidebar-nav .nav-link.is-active {
  background: transparent;
  transform: none;
  box-shadow: none;
}

.nav-indicator {
  position: absolute;
  left: 0;
  right: 0;
  border-radius: 12px;
  background: var(--p-primary-color);
  box-shadow: 0 4px 12px rgba(var(--p-primary-color-rgb), 0.2);
  z-index: 0;
  pointer-events: none;
}

/* No transition on first render; enable after initial positioning */
.nav-indicator-animated {
  transition:
    top 0.3s cubic-bezier(0.4, 0, 0.2, 1),
    height 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.dark .nav-indicator {
  box-shadow: 0 4px 12px rgba(var(--p-primary-color-rgb), 0.15);
}
</style>
