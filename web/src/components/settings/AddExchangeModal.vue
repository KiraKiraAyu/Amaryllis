<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue"
import Dialog from "primevue/dialog"
import Select from "primevue/select"
import InputText from "primevue/inputtext"
import Password from "primevue/password"
import Checkbox from "primevue/checkbox"
import Button from "primevue/button"

import { getSupportedExchangesApi } from "@/api/catalog"
import { createExchangeApi } from "@/api/exchanges"
import type { CreateExchangeRequest } from "@/types/exchanges"
import type { SupportedExchangePayload } from "@/types/public"

const emit = defineEmits<{
  close: []
  created: []
}>()

const addingEx = ref(false)
const supportedExchanges = ref<SupportedExchangePayload[]>([])
const newEx = ref<CreateExchangeRequest>({
  exchange_type: "binance",
  account_name: "",
  api_key: "",
  secret_key: "",
  testnet: true,
})

const exchangeOptions = computed(() => 
  supportedExchanges.value.map(e => ({
    label: `${e.name} (${e.type})`,
    value: e.id
  }))
)

const requiresPassphrase = computed(() =>
  ["okx", "bitget"].includes(newEx.value.exchange_type),
)
const isWalletBased = computed(() =>
  ["hyperliquid", "aster"].includes(newEx.value.exchange_type),
)
const usesApiCredentials = computed(() => !isWalletBased.value)

const walletLabels = computed(() => {
  if (newEx.value.exchange_type === "aster") {
    return {
      address: "Main Wallet Address",
      addressPlaceholder: "0x… connected wallet (user)",
      key: "API Wallet Private Key",
      keyPlaceholder: "0x… from Authorize new API wallet",
    }
  }
  return {
    address: "Wallet Address",
    addressPlaceholder: "0x…",
    key: "Private Key",
    keyPlaceholder: "private key…",
  }
})

async function addExchange() {
  addingEx.value = true
  try {
    await createExchangeApi({ ...newEx.value, enabled: true })
    emit("created")
    emit("close")
  } finally {
    addingEx.value = false
  }
}

async function loadSupportedExchanges() {
  try {
    supportedExchanges.value = await getSupportedExchangesApi()
    newEx.value.exchange_type = supportedExchanges.value[0]?.id ?? "binance"
  } catch {
    supportedExchanges.value = []
  }
}

onMounted(loadSupportedExchanges)

watch(
  () => newEx.value.exchange_type,
  (exchangeType) => {
    newEx.value.api_key = ""
    newEx.value.secret_key = ""
    newEx.value.passphrase = ""
    newEx.value.hyperliquid_wallet_addr = ""
    if (exchangeType === "aster") {
      newEx.value.testnet = false
    } else if (newEx.value.testnet == null) {
      newEx.value.testnet = true
    }
  },
)
</script>

<template>
  <Dialog
    visible
    modal
    header="Add Exchange"
    :style="{ width: '32rem', maxWidth: '90vw' }"
    @update:visible="emit('close')"
  >
    <div class="flex flex-col gap-4 py-2">
      <div class="flex flex-col gap-2">
        <label class="text-sm font-semibold text-surface-700 dark:text-surface-300">Exchange Type</label>
        <Select
          v-model="newEx.exchange_type"
          :options="exchangeOptions"
          optionLabel="label"
          optionValue="value"
          class="w-full"
        />
      </div>

      <div class="flex flex-col gap-2">
        <label class="text-sm font-semibold text-surface-700 dark:text-surface-300">Account Name</label>
        <InputText v-model="newEx.account_name" placeholder="My Binance" />
      </div>

      <div v-if="usesApiCredentials" class="flex flex-col gap-2">
        <label class="text-sm font-semibold text-surface-700 dark:text-surface-300">API Key</label>
        <InputText v-model="newEx.api_key" placeholder="api key…" />
      </div>

      <div v-if="usesApiCredentials" class="flex flex-col gap-2">
        <label class="text-sm font-semibold text-surface-700 dark:text-surface-300">Secret Key</label>
        <Password
          v-model="newEx.secret_key"
          placeholder="secret…"
          toggleMask
          :feedback="false"
          fluid
        />
      </div>

      <div v-if="requiresPassphrase" class="flex flex-col gap-2">
        <label class="text-sm font-semibold text-surface-700 dark:text-surface-300">Passphrase</label>
        <Password
          v-model="newEx.passphrase"
          placeholder="passphrase…"
          toggleMask
          :feedback="false"
          fluid
        />
      </div>

      <div v-if="isWalletBased" class="flex flex-col gap-2">
        <label class="text-sm font-semibold text-surface-700 dark:text-surface-300">{{ walletLabels.address }}</label>
        <InputText
          v-model="newEx.hyperliquid_wallet_addr"
          :placeholder="walletLabels.addressPlaceholder"
        />
      </div>

      <div v-if="isWalletBased" class="flex flex-col gap-2">
        <label class="text-sm font-semibold text-surface-700 dark:text-surface-300">{{ walletLabels.key }}</label>
        <Password
          v-model="newEx.secret_key"
          :placeholder="walletLabels.keyPlaceholder"
          toggleMask
          :feedback="false"
          fluid
        />
      </div>

      <div class="flex items-center gap-2 mt-2">
        <Checkbox v-model="newEx.testnet" inputId="testnet" :binary="true" />
        <label for="testnet" class="text-sm cursor-pointer text-surface-700 dark:text-surface-300">Use testnet</label>
      </div>
    </div>

    <template #footer>
      <div class="flex justify-end gap-2 mt-4">
        <Button label="Cancel" icon="pi pi-times" severity="secondary" @click="emit('close')" />
        <Button :label="addingEx ? 'Adding…' : 'Add'" icon="pi pi-check" :loading="addingEx" @click="addExchange" />
      </div>
    </template>
  </Dialog>
</template>
