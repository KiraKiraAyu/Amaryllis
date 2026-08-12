<script setup lang="ts">
import { ref, computed } from "vue"
import Tabs from "primevue/tabs"
import TabList from "primevue/tablist"
import Tab from "primevue/tab"
import GeneralTab from "@/components/settings/GeneralTab.vue"
import AIModelsTab from "@/components/settings/AIModelsTab.vue"
import ExchangeAccountsTab from "@/components/settings/ExchangeAccountsTab.vue"
import SecurityTab from "@/components/settings/SecurityTab.vue"
import SlideTransition from "@/components/SlideTransition.vue"
import { useCarouselTransition } from "@/composables/useCarouselTransition"

const activeTab = ref("0")
const step = computed(() => Number(activeTab.value))
const { direction } = useCarouselTransition(step)
</script>

<template>
  <div class="flex flex-col gap-6">
    <div>
      <h1 class="mt-1 text-3xl font-bold text-surface-900 dark:text-white">
        Settings
      </h1>
      <p class="mt-1 text-sm text-surface-500 dark:text-surface-400">
        Configure general preferences, exchanges, AI models, and account
        security
      </p>
    </div>

    <Tabs v-model:value="activeTab">
      <TabList>
        <Tab value="0">General</Tab>
        <Tab value="1">AI Models</Tab>
        <Tab value="2">Exchanges</Tab>
        <Tab value="3">Security</Tab>
      </TabList>

      <div class="relative overflow-hidden mt-4">
        <SlideTransition :direction="direction">
          <div v-if="step === 0" key="general" class="w-full">
            <GeneralTab />
          </div>
          <div v-else-if="step === 1" key="ai-models" class="w-full">
            <AIModelsTab />
          </div>
          <div v-else-if="step === 2" key="exchanges" class="w-full">
            <ExchangeAccountsTab />
          </div>
          <div v-else-if="step === 3" key="security" class="w-full">
            <SecurityTab />
          </div>
        </SlideTransition>
      </div>
    </Tabs>
  </div>
</template>
