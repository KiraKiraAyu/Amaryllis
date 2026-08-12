<script setup lang="ts">
import { onMounted, ref } from "vue"
import Button from "primevue/button"
import AddExchangeModal from "@/components/settings/AddExchangeModal.vue"
import EditExchangeModal from "@/components/settings/EditExchangeModal.vue"
import { deleteExchangeApi, getExchangeConfigsApi } from "@/api/exchanges"
import type { SafeExchangeConfig } from "@/types/exchanges"

const exchanges = ref<SafeExchangeConfig[]>([])
const showAddExchange = ref(false)
const editingExchange = ref<SafeExchangeConfig | null>(null)

async function loadExchanges() {
  try {
    const data = await getExchangeConfigsApi()
    exchanges.value = Array.isArray(data) ? data : []
  } catch {
    /* ignore */
  }
}

async function deleteExchange(id: string) {
  if (!confirm("Delete this exchange?")) return
  await deleteExchangeApi(id)
  await loadExchanges()
}

onMounted(loadExchanges)
</script>

<template>
  <div class="flex flex-col gap-4">
    <div>
      <div class="flex items-center justify-between mb-4">
        <h2 class="font-bold text-lg text-surface-900 dark:text-white">
          Exchange Accounts
        </h2>
        <Button
          label="Add Exchange"
          icon="pi pi-plus"
          size="small"
          @click="showAddExchange = true"
        />
      </div>
      <div class="flex flex-col gap-3">
        <div
          v-for="ex in exchanges"
          :key="ex.id"
          class="p-4 rounded-xl bg-surface-0 dark:bg-surface-900 border border-surface-200 dark:border-surface-800"
        >
          <div class="flex items-center justify-between">
            <div>
              <h3 class="font-bold text-surface-900 dark:text-surface-100">
                {{ ex.account_name || ex.exchange_type }}
              </h3>
              <p
                class="text-xs text-surface-500 font-medium tracking-wide uppercase mt-0.5"
              >
                {{ ex.exchange_type }}
              </p>
            </div>
            <div class="flex items-center gap-3">
              <span
                class="text-xs font-bold uppercase tracking-wide px-2 py-1 rounded bg-surface-100 text-surface-600 dark:bg-surface-800 dark:text-surface-400"
              >
                {{ ex.testnet ? "Testnet" : "Live" }}
              </span>
              <span
                class="text-xs font-bold uppercase tracking-wide px-2 py-1 rounded"
                :class="
                  ex.enabled
                    ? 'bg-emerald-100 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400'
                    : 'bg-surface-100 text-surface-600 dark:bg-surface-800 dark:text-surface-400'
                "
              >
                {{ ex.enabled ? "Active" : "Disabled" }}
              </span>
              <Button
                icon="pi pi-pencil"
                severity="info"
                variant="text"
                rounded
                aria-label="Edit"
                @click="editingExchange = ex"
              />
              <Button
                icon="pi pi-trash"
                severity="danger"
                variant="text"
                rounded
                aria-label="Delete"
                @click="deleteExchange(ex.id)"
              />
            </div>
          </div>
        </div>
        <div
          v-if="exchanges.length === 0"
          class="text-center py-12 text-sm text-surface-500"
        >
          No exchanges configured. Add one to start trading.
        </div>
      </div>
    </div>
  </div>

  <AddExchangeModal
    v-if="showAddExchange"
    @close="showAddExchange = false"
    @created="loadExchanges"
  />

  <EditExchangeModal
    v-if="editingExchange"
    :exchange="editingExchange"
    @close="editingExchange = null"
    @updated="loadExchanges"
  />
</template>
