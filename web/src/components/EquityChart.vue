<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from "vue"
import {
  AreaSeries,
  createChart,
  ColorType,
  type IChartApi,
  type ISeriesApi,
  type UTCTimestamp,
} from "lightweight-charts"

const props = withDefaults(
  defineProps<{
    data: { time: number; value: number }[]
    height?: number
    color?: string
  }>(),
  {
    height: 240,
    color: "#e2528a",
  },
)

const chartEl = ref<HTMLElement | null>(null)
let chart: IChartApi | null = null
let series: ISeriesApi<"Area"> | null = null

onMounted(() => {
  if (!chartEl.value) return
  chart = createChart(chartEl.value, {
    layout: {
      background: { type: ColorType.Solid, color: "transparent" },
      textColor: "#7a7091",
      fontFamily: "Outfit, sans-serif",
      fontSize: 11,
    },
    grid: {
      vertLines: { color: "#ffffff08" },
      horzLines: { color: "#ffffff08" },
    },
    rightPriceScale: { borderColor: "#ffffff10" },
    timeScale: { borderColor: "#ffffff10", timeVisible: true },
    handleScroll: true,
    handleScale: true,
  })

  series = chart.addSeries(AreaSeries, {
    lineColor: props.color,
    topColor: props.color + "55",
    bottomColor: props.color + "00",
    lineWidth: 2,
    priceFormat: { type: "price", precision: 2, minMove: 0.01 },
  })

  if (props.data.length) feed(props.data)
  chart.timeScale().fitContent()
  window.addEventListener("resize", onResize)
})

onUnmounted(() => {
  window.removeEventListener("resize", onResize)
  chart?.remove()
})

function feed(arr: { time: number; value: number }[]) {
  if (!series) return
  const dedup = new Map<number, number>()
  for (const d of arr) {
    if (d.time > 0) dedup.set(d.time, d.value)
  }
  const sorted = [...dedup]
    .sort((a, b) => a[0] - b[0])
    .map(([time, value]) => ({ time: time as UTCTimestamp, value }))
  series.setData(sorted)
  chart?.timeScale().fitContent()
}

function onResize() {
  if (chart && chartEl.value)
    chart.applyOptions({ width: chartEl.value.clientWidth })
}

watch(() => props.data, feed, { deep: true })
</script>

<template>
  <div ref="chartEl" class="w-full" :style="{ height: height + 'px' }"></div>
</template>
