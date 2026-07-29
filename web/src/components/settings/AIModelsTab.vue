<script setup lang="ts">
import AIModelDialog from "@/components/settings/ai-models/AIModelDialog.vue"
import AIModelList from "@/components/settings/ai-models/AIModelList.vue"
import AIProviderForm from "@/components/settings/ai-models/AIProviderForm.vue"
import AIProviderList from "@/components/settings/ai-models/AIProviderList.vue"
import ProviderCheckModal from "@/components/settings/ai-models/ProviderCheckModal.vue"
import { useAIModelsSettings } from "@/composables/useAIModelsSettings"

const {
  activeProvider,
  addProvider,
  addRemoteModel,
  apiCategories,
  apiCategoryLabel,
  checkMessage,
  checkingProvider,
  fetchRemoteModels,
  fetchingRemoteModels,
  hasModel,
  modelDialogOpen,
  modelDialogIsNew,
  modelDraft,
  openProviderCheckModal,
  providers,
  providerCheckModalOpen,
  providerCheckModelId,
  providerKey,
  providerLabel,
  remoteModels,
  removeModel,
  removeProvider,
  saveModels,
  saveModelDraft,
  savingModels,
  selectProvider,
  selectedProviderIndex,
  startAddModel,
  startEditModel,
  supportedProviderTypes,
  checkProvider,
} = useAIModelsSettings()
</script>

<template>
  <div class="flex flex-nowrap min-h-155">
    <AIProviderList
      :providers="providers"
      :selected-provider-index="selectedProviderIndex"
      :supported-provider-type-count="supportedProviderTypes.length"
      :api-category-label="apiCategoryLabel"
      :provider-key="providerKey"
      :provider-label="providerLabel"
      @add-provider="addProvider"
      @select-provider="selectProvider"
    />

    <div v-if="activeProvider" class="flex min-w-0 flex-1 flex-col px-4">
      <AIProviderForm
        :provider="activeProvider"
        :api-categories="apiCategories"
        :checking-provider="checkingProvider"
        :saving-models="savingModels"
        :check-message="checkMessage"
        @check-provider="openProviderCheckModal"
        @remove-provider="removeProvider(selectedProviderIndex)"
        @save-models="saveModels"
      />

      <AIModelList
        :provider="activeProvider"
        :remote-models="remoteModels"
        :fetching-remote-models="fetchingRemoteModels"
        :has-model="hasModel"
        @fetch-remote-models="fetchRemoteModels"
        @start-add-model="startAddModel"
        @edit-model="startEditModel"
        @add-remote-model="addRemoteModel"
        @remove-model="removeModel"
      />
    </div>

    <div
      v-else
      class="flex flex-1 items-center justify-center px-4 text-sm text-text-muted"
    >
      Select a provider or add a new one.
    </div>

    <ProviderCheckModal
      v-if="activeProvider"
      v-model:open="providerCheckModalOpen"
      v-model:selected-model-id="providerCheckModelId"
      :provider="activeProvider"
      :checking="checkingProvider"
      @confirm="checkProvider(activeProvider)"
    />

    <AIModelDialog
      v-if="activeProvider"
      v-model:open="modelDialogOpen"
      :model="modelDraft"
      :is-new="modelDialogIsNew"
      @save="saveModelDraft(activeProvider)"
    />
  </div>
</template>
