<script setup lang="ts">
import { computed, ref } from "vue"
import Dialog from "primevue/dialog"
import InputText from "primevue/inputtext"
import Password from "primevue/password"
import Checkbox from "primevue/checkbox"
import Button from "primevue/button"

import { updateExchangeConfigsApi } from "@/api/exchanges"
import type { SafeExchangeConfig, ExchangeConfigPatch } from "@/types/exchanges"

const props = defineProps<{
  exchange: SafeExchangeConfig
}>()

const emit = defineEmits<{
  close: []
  updated: []
}>()

const saving = ref(false)
const form = ref({
  enabled: props.exchange.enabled,
  testnet: props.exchange.testnet,
  hyperliquid_wallet_addr: props.exchange.hyperliquidWalletAddr || "",
  api_key: "",
  secret_key: "",
  passphrase: "",
})

const isWalletBased = computed(() =>
  ["hyperliquid", "aster"].includes(props.exchange.exchange_type),
)
const requiresPassphrase = computed(() =>
  ["okx", "bitget"].includes(props.exchange.exchange_type),
)

const walletLabels = computed(() => {
  if (props.exchange.exchange_type === "aster") {
    return {
      address: "Main Wallet Address (Login Wallet)",
      addressPlaceholder: "0x… your MetaMask login address (user)",
      key: "API Wallet Private Key",
      keyPlaceholder: "Leave empty to keep current, or enter 64 hex chars",
    }
  }
  return {
    address: "Wallet Address",
    addressPlaceholder: "0x…",
    key: "Private Key",
    keyPlaceholder: "Leave empty to keep current",
  }
})

const walletAddrError = computed(() => {
  if (!isWalletBased.value || !form.value.hyperliquid_wallet_addr) return ""
  const hex = form.value.hyperliquid_wallet_addr.trim().replace(/^0x/i, "")
  if (!/^[0-9a-fA-F]*$/.test(hex))
    return "Wallet address contains invalid characters"
  if (hex.length !== 40)
    return `Wallet address must be 40 hex chars (20 bytes), got ${hex.length} chars`
  return ""
})

const privateKeyError = computed(() => {
  if (!isWalletBased.value || !form.value.secret_key) return ""
  const hex = form.value.secret_key.trim().replace(/^0x/i, "")
  if (!/^[0-9a-fA-F]*$/.test(hex))
    return "Private key contains invalid characters"
  if (hex.length === 40)
    return "This looks like a wallet address (40 chars), not a private key. Private keys are 64 hex chars."
  if (hex.length !== 64)
    return `Private key must be 64 hex chars (32 bytes), got ${hex.length} chars`
  return ""
})

const canSubmit = computed(() => {
  if (isWalletBased.value) {
    return !privateKeyError.value && !walletAddrError.value
  }
  return true
})

const asterHelpVisible = computed(
  () => props.exchange.exchange_type === "aster",
)

const asterProApiUrl = computed(() =>
  form.value.testnet
    ? "https://www.asterdex-testnet.com/en/api-wallet"
    : "https://www.asterdex.com/en/api-wallet",
)

async function save() {
  if (!canSubmit.value) return
  saving.value = true
  try {
    const patch: ExchangeConfigPatch = {
      enabled: form.value.enabled,
      testnet: form.value.testnet,
      hyperliquid_wallet_addr: form.value.hyperliquid_wallet_addr.trim(),
    }
    if (form.value.api_key.trim()) patch.api_key = form.value.api_key.trim()
    if (form.value.secret_key.trim())
      patch.secret_key = form.value.secret_key.trim()
    if (form.value.passphrase.trim())
      patch.passphrase = form.value.passphrase.trim()

    await updateExchangeConfigsApi({
      exchanges: { [props.exchange.id]: patch },
    })
    emit("updated")
    emit("close")
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <Dialog
    visible
    modal
    :header="`Edit ${exchange.name || exchange.exchange_type}`"
    :style="{ width: '32rem', maxWidth: '90vw' }"
    @update:visible="emit('close')"
  >
    <div class="flex flex-col gap-4 py-2">
      <div class="flex items-center gap-2">
        <Checkbox
          v-model="form.enabled"
          inputId="edit-enabled"
          :binary="true"
        />
        <label
          for="edit-enabled"
          class="text-sm cursor-pointer text-surface-700 dark:text-surface-300"
          >Enabled</label
        >
      </div>

      <div v-if="!isWalletBased" class="flex flex-col gap-2">
        <label
          class="text-sm font-semibold text-surface-700 dark:text-surface-300"
          >API Key</label
        >
        <InputText
          v-model="form.api_key"
          placeholder="Leave empty to keep current"
        />
      </div>

      <div v-if="!isWalletBased" class="flex flex-col gap-2">
        <label
          class="text-sm font-semibold text-surface-700 dark:text-surface-300"
          >Secret Key</label
        >
        <Password
          v-model="form.secret_key"
          placeholder="Leave empty to keep current"
          toggleMask
          :feedback="false"
          fluid
        />
      </div>

      <div v-if="requiresPassphrase" class="flex flex-col gap-2">
        <label
          class="text-sm font-semibold text-surface-700 dark:text-surface-300"
          >Passphrase</label
        >
        <Password
          v-model="form.passphrase"
          placeholder="Leave empty to keep current"
          toggleMask
          :feedback="false"
          fluid
        />
      </div>

      <div v-if="isWalletBased" class="flex flex-col gap-2">
        <label
          class="text-sm font-semibold text-surface-700 dark:text-surface-300"
          >{{ walletLabels.address }}</label
        >
        <InputText
          v-model="form.hyperliquid_wallet_addr"
          :placeholder="walletLabels.addressPlaceholder"
          :invalid="!!walletAddrError"
        />
        <small v-if="walletAddrError" class="text-red-500 text-xs">{{
          walletAddrError
        }}</small>
      </div>

      <div v-if="isWalletBased" class="flex flex-col gap-2">
        <label
          class="text-sm font-semibold text-surface-700 dark:text-surface-300"
          >{{ walletLabels.key }}</label
        >
        <Password
          v-model="form.secret_key"
          :placeholder="walletLabels.keyPlaceholder"
          toggleMask
          :feedback="false"
          fluid
          :invalid="!!privateKeyError"
        />
        <small v-if="privateKeyError" class="text-red-500 text-xs">{{
          privateKeyError
        }}</small>
      </div>

      <div
        v-if="asterHelpVisible"
        class="text-xs text-surface-500 dark:text-surface-400 bg-surface-50 dark:bg-surface-800 rounded p-2"
      >
        Aster requires two values from
        <a :href="asterProApiUrl" target="_blank" class="text-primary underline"
          >Pro API page</a
        >:<br />
        1. <strong>Main Wallet Address</strong>: your MetaMask login address (42
        chars with 0x)<br />
        2. <strong>API Wallet Private Key</strong>: the private key of the API
        wallet you created (66 chars with 0x)
        <span v-if="form.testnet" class="block mt-1 font-semibold">
          Testnet: make sure to create the API wallet on the testnet Pro API
          page, not the mainnet one.
        </span>
      </div>

      <div class="flex items-center gap-2 mt-2">
        <Checkbox
          v-model="form.testnet"
          inputId="edit-testnet"
          :binary="true"
        />
        <label
          for="edit-testnet"
          class="text-sm cursor-pointer text-surface-700 dark:text-surface-300"
          >Use testnet</label
        >
      </div>
    </div>

    <template #footer>
      <div class="flex justify-end gap-2 mt-4">
        <Button
          label="Cancel"
          icon="pi pi-times"
          severity="secondary"
          @click="emit('close')"
        />
        <Button
          :label="saving ? 'Saving…' : 'Save'"
          icon="pi pi-check"
          :loading="saving"
          :disabled="!canSubmit"
          @click="save"
        />
      </div>
    </template>
  </Dialog>
</template>
