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

const asterHelpVisible = computed(() => newEx.value.exchange_type === "aster")

const asterProApiUrl = computed(() =>
  newEx.value.testnet
    ? "https://www.asterdex-testnet.com/en/api-wallet"
    : "https://www.asterdex.com/en/api-wallet",
)

const walletLabels = computed(() => {
  if (newEx.value.exchange_type === "aster") {
    return {
      address: "Main Wallet Address (Login Wallet)",
      addressPlaceholder: "0x… your MetaMask login address (user)",
      key: "API Wallet Private Key",
      keyPlaceholder: "0x… 64 hex chars, from Aster Pro API page",
    }
  }
  return {
    address: "Wallet Address",
    addressPlaceholder: "0x…",
    key: "Private Key",
    keyPlaceholder: "private key…",
  }
})

const privateKeyError = computed(() => {
  if (!isWalletBased.value || !newEx.value.secret_key) return ""
  const hex = newEx.value.secret_key.trim().replace(/^0x/i, "")
  if (!/^[0-9a-fA-F]*$/.test(hex)) return "Private key contains invalid characters"
  if (hex.length === 40) return "This looks like a wallet address (40 chars), not a private key. Private keys are 64 hex chars."
  if (hex.length !== 64) return `Private key must be 64 hex chars (32 bytes), got ${hex.length} chars`
  return ""
})

const walletAddrError = computed(() => {
  if (!isWalletBased.value || !newEx.value.hyperliquid_wallet_addr) return ""
  const hex = newEx.value.hyperliquid_wallet_addr.trim().replace(/^0x/i, "")
  if (!/^[0-9a-fA-F]*$/.test(hex)) return "Wallet address contains invalid characters"
  if (hex.length !== 40) return `Wallet address must be 40 hex chars (20 bytes), got ${hex.length} chars`
  return ""
})

const canSubmit = computed(() => {
  if (isWalletBased.value) {
    return !privateKeyError.value && !walletAddrError.value
  }
  return true
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
          :invalid="!!walletAddrError"
        />
        <small v-if="walletAddrError" class="text-red-500 text-xs">{{ walletAddrError }}</small>
      </div>

      <div v-if="isWalletBased" class="flex flex-col gap-2">
        <label class="text-sm font-semibold text-surface-700 dark:text-surface-300">{{ walletLabels.key }}</label>
        <Password
          v-model="newEx.secret_key"
          :placeholder="walletLabels.keyPlaceholder"
          toggleMask
          :feedback="false"
          fluid
          :invalid="!!privateKeyError"
        />
        <small v-if="privateKeyError" class="text-red-500 text-xs">{{ privateKeyError }}</small>
      </div>

      <div v-if="asterHelpVisible" class="text-xs text-surface-500 dark:text-surface-400 bg-surface-50 dark:bg-surface-800 rounded p-2">
        Aster requires two values from
        <a :href="asterProApiUrl" target="_blank" class="text-primary underline">Pro API page</a>:<br />
        1. <strong>Main Wallet Address</strong>: your MetaMask login address (42 chars with 0x)<br />
        2. <strong>API Wallet Private Key</strong>: the private key of the API wallet you created (66 chars with 0x)
        <span v-if="newEx.testnet" class="block mt-1 text-amber-500 font-semibold">
          Testnet: make sure to create the API wallet on the testnet Pro API page, not the mainnet one.
        </span>
      </div>

      <div class="flex items-center gap-2 mt-2">
        <Checkbox v-model="newEx.testnet" inputId="testnet" :binary="true" />
        <label for="testnet" class="text-sm cursor-pointer text-surface-700 dark:text-surface-300">Use testnet</label>
      </div>
    </div>

    <template #footer>
      <div class="flex justify-end gap-2 mt-4">
        <Button label="Cancel" icon="pi pi-times" severity="secondary" @click="emit('close')" />
        <Button :label="addingEx ? 'Adding…' : 'Add'" icon="pi pi-check" :loading="addingEx" :disabled="!canSubmit" @click="addExchange" />
      </div>
    </template>
  </Dialog>
</template>
