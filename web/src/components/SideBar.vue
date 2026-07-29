<script setup lang="ts">
import { Icon } from "@iconify/vue"
import BaseButton from "@/components/universal/BaseButton.vue"
import NavItem from "@/components/NavItem.vue"
import { useAuthStore } from "@/stores/auth"
import { useRealtimeStore } from "@/stores/realtime"

const authStore = useAuthStore()
const realtime = useRealtimeStore()

const navItems = [
  { to: "/", icon: "ic:round-dashboard", label: "Dashboard" },
  { to: "/data", icon: "ic:round-bar-chart", label: "Market Data" },
  { to: "/strategy", icon: "ic:round-settings", label: "Strategy" },
  { to: "/backtest", icon: "ic:round-psychology", label: "Backtest" },
  { to: "/debate", icon: "ic:sharp-sentiment-satisfied", label: "AI Debate" },
  {
    to: "/competition",
    icon: "ic:baseline-accessible-forward",
    label: "Competition",
  },
  { to: "/monitor", icon: "ic:round-thermostat", label: "Monitor" },
  { to: "/settings", icon: "ic:round-settings", label: "Settings" },
]
</script>

<template>
  <aside class="flex flex-col w-55 shrink-0 border-r">
    <div class="flex items-center gap-2 px-5 py-5 border-b">
      <div
        class="w-8 h-8 rounded-lg flex items-center justify-center text-sm font-black"
      >
        A
      </div>
      <span class="font-black text-lg tracking-tight">QUANTAURA</span>
    </div>

    <nav class="flex-1 px-3 py-4 flex flex-col gap-1">
      <NavItem
        v-for="item in navItems"
        :key="item.to"
        :to="item.to"
        :icon="item.icon"
        :label="item.label"
      />
    </nav>

    <div class="px-3 py-4 flex flex-col gap-2 border-t">
      <div class="flex items-center gap-2 px-3 py-2">
        <span
          class="w-2 h-2 rounded-full shrink-0 transition-colors"
          :class="
            realtime.isConnected
              ? 'bg-success animate-pulse'
              : 'bg-text-muted'
          "
        ></span>
        <span class="text-xs text-text-muted">
          {{ realtime.isConnected ? "Live" : "Offline" }}
        </span>
      </div>
      <div class="flex items-center gap-2 px-3 py-2 rounded-lg">
        <div
          class="w-6 h-6 rounded-full flex items-center justify-center text-xs font-bold"
        >
          {{ (authStore.username || "?").charAt(0).toUpperCase() }}
        </div>
        <span class="flex-1 text-xs font-medium truncate">
          {{ authStore.username }}
        </span>
        <BaseButton
          @click="authStore.logout()"
          class="text-xs transition-colors hover:text-error"
          title="Sign out"
        >
          <Icon
            icon="ic:round-logout"
            class="inline-block text-base align-[-0.125em]"
          />
        </BaseButton>
      </div>
    </div>
  </aside>
</template>
