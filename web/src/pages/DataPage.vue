<script setup lang="ts">
import { ref, onMounted } from "vue"
import { getExchangeSymbolsApi, getKlinesApi } from "@/api/market"
import MarketChartPanel from "@/components/data/MarketChartPanel.vue"
import MarketDataHeader from "@/components/data/MarketDataHeader.vue"
import type { CandlePoint } from "@/types/data-ui"

const loading = ref(true)
const symbols = ref<string[]>(["BTCUSDT", "ETHUSDT", "SOLUSDT", "BNBUSDT"])
const activeSymbol = ref("BTCUSDT")
const activeInterval = ref("15m")
const klines = ref<CandlePoint[]>([])

async function loadSymbols() {
  try {
    const data = await getExchangeSymbolsApi()
    if (Array.isArray(data?.symbols)) {
      symbols.value = data.symbols.map((item) => item.symbol)
      if (
        !symbols.value.includes(activeSymbol.value) &&
        symbols.value.length > 0
      ) {
        activeSymbol.value = symbols.value[0]!
      }
    }
  } catch (e) {
    //
  }
}

async function loadKlines() {
  loading.value = true
  try {
    const kdata = await getKlinesApi({
      symbol: activeSymbol.value,
      interval: activeInterval.value,
    })
    klines.value = kdata.map((k) => ({
      time: Math.floor(k.openTime / 1000) as CandlePoint["time"],
      open: Number(k.open),
      high: Number(k.high),
      low: Number(k.low),
      close: Number(k.close),
    }))
  } catch (e) {
    klines.value = []
  } finally {
    loading.value = false
  }
}

onMounted(async () => {
  await loadSymbols()
  await loadKlines()
})
</script>

<template>
  <div class="flex flex-col gap-6 h-full min-h-0">
    <MarketDataHeader
      v-model:symbol="activeSymbol"
      v-model:interval="activeInterval"
      :symbols="symbols"
      @refresh="loadKlines"
      class="shrink-0"
    />

    <MarketChartPanel
      :loading="loading"
      :data="klines"
      :active-symbol="activeSymbol"
      class="flex-1 min-h-0"
    />
  </div>
</template>
