<script setup lang="ts">
import Card from "primevue/card"
import Button from "primevue/button"
import type { EditableStrategy } from "@/types/strategy-ui"
import { computed } from "vue"

const props = defineProps<{
  strategy: EditableStrategy
  duplicating?: boolean
  testRunLoading?: boolean
  previewLoading?: boolean
}>()

const emit = defineEmits<{
  duplicate: []
  delete: []
  edit: []
  test: []
  preview: []
}>()

const config = computed(() => props.strategy.config || {})
const symbols = computed(() => config.value.symbols || [])
const maxPositions = computed(() => config.value.max_positions ?? 5)
const promptVariant = computed(() => config.value.prompt_variant ?? "balanced")

// TP/SL display
const tpSl = computed(() => config.value.tp_sl)
const takeProfit = computed(() => tpSl.value?.take_profit)
const stopLoss = computed(() => tpSl.value?.stop_loss)
</script>

<template>
  <Card class="border border-surface-200 dark:border-surface-800 bg-surface-0 dark:bg-surface-900 shadow-none!">
    <template #content>
      <!-- Header -->
      <div class="flex items-start gap-4 mb-6 flex-wrap justify-between">
        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-2.5 mb-1.5 flex-wrap">
            <h2 class="font-bold text-xl text-surface-900 dark:text-white truncate">
              {{ strategy.name }}
            </h2>
          </div>
          <p class="text-sm text-surface-500 dark:text-surface-400">
            {{ strategy.description || 'No description provided.' }}
          </p>
        </div>

        <div class="flex gap-2 shrink-0">
          <Button
            icon="pi pi-pencil"
            label="Edit"
            severity="secondary"
            @click="emit('edit')"
            class="rounded-xl h-10 cursor-pointer"
          />
          <Button
            icon="pi pi-copy"
            label="Duplicate"
            severity="secondary"
            @click="emit('duplicate')"
            :loading="duplicating"
            class="rounded-xl h-10 cursor-pointer"
          />
          <Button
            icon="pi pi-trash"
            label="Delete"
            severity="danger"
            @click="emit('delete')"
            class="rounded-xl h-10 cursor-pointer bg-rose-500! border-rose-500! text-white!"
          />
        </div>
      </div>

      <!-- Strategy Info Bar -->
      <div class="flex items-center gap-8 mb-6 p-4 rounded-2xl bg-surface-50 dark:bg-surface-950 border border-surface-200 dark:border-surface-800">
        <div class="flex items-center gap-3">
          <div class="h-9 w-9 rounded-xl bg-primary-500/10 dark:bg-primary-500/20 flex items-center justify-center text-primary">
            <span class="pi pi-compass text-base"></span>
          </div>
          <div class="flex flex-col">
            <span class="text-xs text-surface-400 dark:text-surface-500 font-bold uppercase tracking-wider">Prompt Variant</span>
            <span class="text-sm font-bold text-surface-900 dark:text-white capitalize">{{ promptVariant }}</span>
          </div>
        </div>

        <div class="w-px h-8 bg-surface-200 dark:bg-surface-800"></div>

        <div class="flex items-center gap-3">
          <div class="h-9 w-9 rounded-xl bg-blue-500/10 dark:bg-blue-500/20 flex items-center justify-center text-blue-500">
            <span class="pi pi-list text-base"></span>
          </div>
          <div class="flex flex-col">
            <span class="text-xs text-surface-400 dark:text-surface-500 font-bold uppercase tracking-wider">Max Positions</span>
            <span class="text-sm font-bold text-surface-900 dark:text-white font-mono">{{ maxPositions }}</span>
          </div>
        </div>
      </div>

      <!-- Target Symbols List -->
      <div class="mb-6">
        <h3 class="font-bold text-sm text-surface-900 dark:text-white mb-3">Trading Target Symbols</h3>
        <div class="overflow-x-auto border border-surface-200 dark:border-surface-800 rounded-2xl">
          <table class="w-full text-left border-collapse">
            <thead>
              <tr class="bg-surface-50 dark:bg-surface-950 border-b border-surface-200 dark:border-surface-800">
                <th class="p-3 text-xs font-bold text-surface-500 uppercase tracking-wider">Symbol</th>
                <th class="p-3 text-xs font-bold text-surface-500 uppercase tracking-wider">Leverage</th>
                <th class="p-3 text-xs font-bold text-surface-500 uppercase tracking-wider">Cost Mode</th>
                <th class="p-3 text-xs font-bold text-surface-500 uppercase tracking-wider text-right">Cost Setting</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-surface-200 dark:divide-surface-800">
              <tr v-if="symbols.length === 0">
                <td colspan="4" class="p-4 text-center text-sm text-surface-400">
                  No symbols configured.
                </td>
              </tr>
              <tr v-for="item in symbols" :key="item.symbol" class="hover:bg-surface-50/50 dark:hover:bg-surface-950/20">
                <td class="p-3 text-sm font-bold text-surface-900 dark:text-white font-mono">
                  {{ item.symbol }}
                </td>
                <td class="p-3 text-sm text-surface-700 dark:text-surface-300 font-mono">
                  {{ item.leverage }}x
                </td>
                <td class="p-3 text-sm text-surface-600 dark:text-surface-400">
                  <span class="px-2 py-0.5 text-xs rounded-lg font-semibold bg-surface-100 dark:bg-surface-800 border border-surface-200 dark:border-surface-750">
                    {{ item.fixed_cost != null ? 'Fixed' : 'Dynamic' }}
                  </span>
                </td>
                <td class="p-3 text-sm font-mono text-surface-900 dark:text-white text-right">
                  <span v-if="item.fixed_cost != null" class="text-amber-500 font-bold">
                    ${{ item.fixed_cost }} (Fixed)
                  </span>
                  <span v-else class="text-surface-600 dark:text-surface-400">
                    ${{ item.min_cost ?? '5.0' }} - ${{ item.max_cost ?? '∞' }}
                  </span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- Take-Profit / Stop-Loss Rules -->
      <div class="mb-6">
        <h3 class="font-bold text-sm text-surface-900 dark:text-white mb-3">Take-Profit / Stop-Loss Rules</h3>
        <div class="border border-surface-200 dark:border-surface-800 rounded-2xl p-4">
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div class="flex flex-col gap-2 p-3 rounded-xl bg-surface-50 dark:bg-surface-950 border border-surface-200 dark:border-surface-800">
              <div class="flex items-center gap-2">
                <span class="pi pi-arrow-up-right text-emerald-500"></span>
                <span class="text-xs font-bold text-surface-400 uppercase tracking-wider">Take-Profit</span>
                <span class="ml-auto text-xs font-semibold text-surface-500">
                  {{ takeProfit?.mode === 'custom' ? 'Custom AI' : takeProfit?.mode === 'fixed' ? 'Fixed' : 'Not configured' }}
                </span>
              </div>
              <template v-if="takeProfit?.mode === 'custom'">
                <p class="text-sm text-surface-700 dark:text-surface-300 whitespace-pre-wrap">
                  {{ takeProfit.custom_prompt || 'No custom instructions provided.' }}
                </p>
              </template>
              <template v-else-if="takeProfit?.mode === 'fixed'">
                <span class="text-lg font-bold text-emerald-500 font-mono">
                  <template v-if="takeProfit?.pnl_rate != null">
                    +{{ (takeProfit.pnl_rate * 100).toFixed(1) }}%
                  </template>
                  <template v-else>Not configured</template>
                </span>
              </template>
              <span v-else class="text-sm text-surface-400">Not configured</span>
            </div>
            <div class="flex flex-col gap-2 p-3 rounded-xl bg-surface-50 dark:bg-surface-950 border border-surface-200 dark:border-surface-800">
              <div class="flex items-center gap-2">
                <span class="pi pi-arrow-down-right text-rose-500"></span>
                <span class="text-xs font-bold text-surface-400 uppercase tracking-wider">Stop-Loss</span>
                <span class="ml-auto text-xs font-semibold text-surface-500">
                  {{ stopLoss?.mode === 'custom' ? 'Custom AI' : stopLoss?.mode === 'fixed' ? 'Fixed' : 'Not configured' }}
                </span>
              </div>
              <template v-if="stopLoss?.mode === 'custom'">
                <p class="text-sm text-surface-700 dark:text-surface-300 whitespace-pre-wrap">
                  {{ stopLoss.custom_prompt || 'No custom instructions provided.' }}
                </p>
              </template>
              <template v-else-if="stopLoss?.mode === 'fixed'">
                <span class="text-lg font-bold text-rose-500 font-mono">
                  <template v-if="stopLoss?.pnl_rate != null">
                    {{ (stopLoss.pnl_rate * 100).toFixed(1) }}%
                  </template>
                  <template v-else>Not configured</template>
                </span>
              </template>
              <span v-else class="text-sm text-surface-400">Not configured</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Action Buttons -->
      <div class="flex gap-3 mt-4 border-t border-surface-200 dark:border-surface-800 pt-4">
        <Button
          icon="pi pi-sparkles"
          label="Test Run (AI)"
          severity="help"
          @click="emit('test')"
          :loading="testRunLoading"
          class="rounded-xl h-11 cursor-pointer flex-1"
        />
        <Button
          icon="pi pi-eye"
          label="Preview Prompt"
          severity="secondary"
          text
          @click="emit('preview')"
          :loading="previewLoading"
          class="rounded-xl h-11 cursor-pointer"
        />
      </div>
    </template>
  </Card>
</template>
