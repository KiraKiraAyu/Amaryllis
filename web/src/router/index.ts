import { createRouter, createWebHistory } from "vue-router"
import { useAuthStore } from "@/stores/auth"

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/login",
      name: "login",
      component: () => import("@/pages/LoginPage.vue"),
      meta: { public: true },
    },
    {
      path: "/setup",
      name: "setup",
      component: () => import("@/pages/SetupPage.vue"),
      meta: { public: true },
    },
    {
      path: "/",
      component: () => import("@/layout/MainLayout.vue"),
      children: [
        {
          path: "",
          name: "dashboard",
          component: () => import("@/pages/DashboardPage.vue"),
        },
        {
          path: "strategy",
          name: "strategy",
          component: () => import("@/pages/StrategyPage.vue"),
        },
        {
          path: "debate",
          name: "debate",
          component: () => import("@/pages/DebatePage.vue"),
        },
        {
          path: "traders",
          name: "traders",
          component: () => import("@/pages/TradersPage.vue"),
        },
        {
          path: "traders/:id",
          name: "trader-detail",
          component: () => import("@/pages/TraderDetailPage.vue"),
          meta: { depth: 1 },
        },
        {
          path: "traders/:id/backtest",
          name: "trader-backtest",
          component: () => import("@/pages/TraderBacktestPage.vue"),
          meta: { depth: 2 },
        },
        {
          path: "data",
          name: "data",
          component: () => import("@/pages/DataPage.vue"),
        },
        {
          path: "monitor",
          name: "monitor",
          component: () => import("@/pages/SystemMonitorPage.vue"),
        },
        {
          path: "settings",
          name: "settings",
          component: () => import("@/pages/SettingsPage.vue"),
        },
      ],
    },
  ],
})

router.beforeEach(async (to) => {
  const auth = useAuthStore()

  // Probe the server's TOTP configuration state once per session.
  if (auth.configured === null) {
    try {
      await auth.refreshStatus()
    } catch {
      // If the probe fails (e.g. server unreachable), fall through and
      // treat the instance as configured so the login page stays reachable.
      auth.configured = true
    }
  }

  // First-run: no authenticator configured yet — force the setup flow.
  if (auth.configured === false) {
    return to.name === "setup" ? true : { name: "setup" }
  }
  if (to.name === "setup") {
    return { name: auth.isLoggedIn ? "dashboard" : "login" }
  }

  if (!to.meta.public && !auth.isLoggedIn) {
    return { name: "login" }
  }
  if (to.meta.public && auth.isLoggedIn) {
    return { name: "dashboard" }
  }
})

export default router
