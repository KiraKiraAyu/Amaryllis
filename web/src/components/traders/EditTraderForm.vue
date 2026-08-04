<script setup lang="ts">
import Card from "primevue/card"
import InputText from "primevue/inputtext"
import InputNumber from "primevue/inputnumber"
import Select from "primevue/select"
import Button from "primevue/button"
import { ref, onMounted, watch } from "vue"
import { getExchangeConfigsApi } from "@/api/exchanges"
import { getModelConfigsApi } from "@/api/models"
import { getStrategiesApi } from "@/api/strategies"
import { updateTraderApi } from "@/api/trading"
import type { SafeExchangeConfig } from "@/types/exchanges"
import type { ModelConfigPayload } from "@/types/models"
import type { StrategyListPayload, StrategyPayload } from "@/types/strategies"
import type { TraderPayload, UpdateTraderRequest } from "@/types/trading"

const props = defineProps<{
  trader: TraderPayload
}>()

const emit = defineEmits(["close", "updated"])

const loading = ref(false)
const error = ref("")
const exchanges = ref<SafeExchangeConfig[]>([])
const strategies = ref<StrategyPayload[]>([])
const models = ref<{ id: string; label: string }[]>([])
const form = ref({
  name: "",
  exchange_id: "",
  ai_model_id: "",
  strategy_id: "",
  scan_interval_minutes: 60,
  initial_balance: 1000,
})

function syncFormFromTrader() {
  form.value = {
    name: props.trader.name ?? "",
    exchange_id: props.trader.exchange_id ?? "",
    ai_model_id: props.trader.ai_model_id ?? "",
    strategy_id: props.trader.strategy_id ?? "",
    scan_interval_minutes: props.trader.scan_interval_minutes ?? 60,
    initial_balance: props.trader.initial_balance ?? 1000,
  }
}

async function submit() {
  loading.value = true
  error.value = ""
  if (!form.value.ai_model_id) {
    error.value = "Select an AI model first"
    loading.value = false
    return
  }
  try {
    const payload: UpdateTraderRequest = {
      name: form.value.name,
      ai_model_id: form.value.ai_model_id,
      exchange_id: form.value.exchange_id,
      strategy_id: form.value.strategy_id || null,
      scan_interval_minutes: form.value.scan_interval_minutes,
      initial_balance: form.value.initial_balance,
    }
    await updateTraderApi(props.trader.id, payload)
    emit("updated")
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : "Failed to update trader"
  } finally {
    loading.value = false
  }
}

async function loadOptions() {
  const [exRes, stRes, modelRes] = await Promise.all([
    getExchangeConfigsApi().catch((): SafeExchangeConfig[] => []),
    getStrategiesApi().catch(
      (): StrategyListPayload => ({ strategies: [], count: 0 }),
    ),
    getModelConfigsApi().catch((): ModelConfigPayload => ({ providers: [] })),
  ])
  exchanges.value = exRes
  strategies.value = stRes.strategies

  const providerItems = modelRes.providers
  models.value = providerItems
    .filter((provider) => provider.enabled ?? true)
    .flatMap(
      (provider: {
        name?: string
        models?: { id?: string; name?: string }[]
      }) =>
        (provider.models ?? []).map((model) => ({
          id: model.id ?? "",
          label: `${provider.name ?? "Provider"} / ${model.name ?? model.id ?? ""}`,
        })),
    )

  if (
    !form.value.ai_model_id ||
    !models.value.some(
      (model: { id: string }) => model.id === form.value.ai_model_id,
    )
  ) {
    form.value.ai_model_id = models.value[0]?.id ?? ""
  }
}

watch(
  () => props.trader,
  () => {
    syncFormFromTrader()
  },
  { immediate: true },
)

onMounted(loadOptions)
</script>

<template>
  <Card class="border border-surface-200 dark:border-surface-800 bg-surface-0 dark:bg-surface-900 shadow-none!">
    <template #content>
      <div class="flex items-center gap-3 mb-6">
        <h2 class="font-bold text-xl text-surface-900 dark:text-white flex-1 truncate">Edit AI Trader</h2>
      </div>

      <form @submit.prevent="submit" class="flex flex-col gap-5">
        <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <div class="flex flex-col gap-1.5">
            <label class="text-xs font-bold text-surface-500 dark:text-surface-400">Trader Name</label>
            <InputText
              v-model="form.name"
              placeholder="My AI Trader"
              required
              class="h-10 rounded-xl"
            />
          </div>

          <div class="flex flex-col gap-1.5">
            <label class="text-xs font-bold text-surface-500 dark:text-surface-400">Exchange Account</label>
            <Select
              v-model="form.exchange_id"
              :options="exchanges"
              optionLabel="account_name"
              optionValue="id"
              placeholder="Select exchange..."
              required
              class="h-10 rounded-xl flex items-center"
            >
              <template #option="{ option }">
                {{ option.account_name || option.exchange_type }}
              </template>
            </Select>
          </div>

          <div class="flex flex-col gap-1.5">
            <label class="text-xs font-bold text-surface-500 dark:text-surface-400">AI Model</label>
            <Select
              v-model="form.ai_model_id"
              :options="models"
              optionLabel="label"
              optionValue="id"
              placeholder="Select model..."
              required
              class="h-10 rounded-xl flex items-center"
            />
          </div>

          <div class="flex flex-col gap-1.5">
            <label class="text-xs font-bold text-surface-500 dark:text-surface-400">Trading Strategy</label>
            <Select
              v-model="form.strategy_id"
              :options="[{ id: '', name: 'No strategy (Default)' }, ...strategies]"
              optionLabel="name"
              optionValue="id"
              placeholder="Select strategy..."
              class="h-10 rounded-xl flex items-center"
            />
          </div>

          <div class="flex flex-col gap-1.5">
            <label class="text-xs font-bold text-surface-500 dark:text-surface-400">Scan Interval (minutes)</label>
            <InputNumber
              v-model="form.scan_interval_minutes"
              :min="1"
              :max="1440"
              showButtons
              buttonLayout="horizontal"
              class="h-10 rounded-xl"
              inputClass="text-center font-mono"
            />
          </div>

          <div class="flex flex-col gap-1.5">
            <label class="text-xs font-bold text-surface-500 dark:text-surface-400">Budget (USDT)</label>
            <InputNumber
              v-model="form.initial_balance"
              :min="10"
              mode="currency"
              currency="USD"
              locale="en-US"
              class="h-10 rounded-xl"
              inputClass="font-mono"
            />
          </div>
        </div>

        <p v-if="error" class="text-xs text-rose-500 bg-rose-50 dark:bg-rose-950/20 px-3 py-2 rounded-xl">
          {{ error }}
        </p>

        <div class="flex gap-3 mt-3 border-t border-surface-200 dark:border-surface-800 pt-4">
          <Button
            type="submit"
            label="Save Changes"
            icon="pi pi-check"
            :loading="loading"
            class="flex-1 rounded-xl h-11 cursor-pointer"
          />
          <Button
            type="button"
            label="Cancel"
            icon="pi pi-times"
            severity="secondary"
            @click="emit('close')"
            class="rounded-xl h-11 cursor-pointer w-32"
          />
        </div>
      </form>
    </template>
  </Card>
</template>
