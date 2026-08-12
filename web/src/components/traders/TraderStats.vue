<script setup lang="ts">
import StatCard from "@/components/StatCard.vue"

defineProps<{
  total: number
  running: number
  stopped: number
  totalPnl: number
  loading: boolean
}>()
</script>

<template>
  <div class="grid grid-cols-2 gap-4 lg:grid-cols-4">
    <StatCard title="Total Traders" :value="total" :loading="loading" />
    <StatCard title="Active Traders" :value="running" :loading="loading" />
    <StatCard title="Stopped Traders" :value="stopped" :loading="loading" />
    <StatCard
      title="Total PnL"
      :value="totalPnl"
      prefix="$"
      :loading="loading"
    />
  </div>
</template>
