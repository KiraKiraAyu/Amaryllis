<script setup lang="ts">
import Button from "primevue/button"
import PageHeader from "@/components/layout/PageHeader.vue"
import StrategyEditor from "@/components/strategy/StrategyEditor.vue"
import StrategyDetail from "@/components/strategy/StrategyDetail.vue"
import StrategyList from "@/components/strategy/StrategyList.vue"
import StrategyPromptPreview from "@/components/strategy/StrategyPromptPreview.vue"
import StrategyTestResult from "@/components/strategy/StrategyTestResult.vue"
import { useStrategyPage } from "@/composables/useStrategyPage"

const {
  createNew,
  deleteStrategy,
  duplicateStrategy,
  duplicating,
  loading,
  previewLoading,
  previewPrompt,
  previewPromptText,
  runTest,
  saveStrategy,
  saving,
  selected,
  strategies,
  testResult,
  testRunLoading,
  isEditing,
  startEdit,
  cancelEdit,
  selectStrategy,
  backToList,
} = useStrategyPage()
</script>

<template>
  <div class="flex flex-col gap-6 w-full">
    <PageHeader
      title="Strategy Studio"
      description="Create and manage trading strategies"
    >
      <template #actions>
        <Button label="New Strategy" icon="pi pi-plus" @click="createNew" />
      </template>
    </PageHeader>

    <div class="relative w-full overflow-hidden min-h-125">
      <Transition name="full-page-slide" mode="out-in">
        <!-- View 1: Strategy List Screen (Full Screen Grid) -->
        <div v-if="!selected && !isEditing" class="w-full" key="list-view">
          <StrategyList
            :strategies="strategies"
            :loading="loading"
            @select="selectStrategy"
          />
        </div>

        <!-- View 2: Strategy Detail Screen (Full Screen Detail Panel with Back Button) -->
        <div v-else-if="selected && !isEditing" class="flex flex-col gap-4 w-full" key="detail-view">
          <div class="flex items-center">
            <Button
              icon="pi pi-arrow-left"
              label="Back to List"
              text
              severity="secondary"
              @click="backToList"
              class="rounded-xl h-10 cursor-pointer"
            />
          </div>
          <StrategyDetail
            :strategy="selected"
            :duplicating="duplicating"
            :test-run-loading="testRunLoading"
            :preview-loading="previewLoading"
            @duplicate="duplicateStrategy"
            @delete="deleteStrategy"
            @edit="startEdit"
            @test="runTest"
            @preview="previewPrompt"
          />

          <StrategyPromptPreview
            v-if="previewPromptText"
            :preview="previewPromptText"
            @close="previewPromptText = null"
          />
          <StrategyTestResult v-if="testResult" :result="testResult" />
        </div>

        <!-- View 3: Strategy Editor Screen (Full Width Editor) -->
        <div v-else-if="isEditing" class="w-full" key="edit-view">
          <StrategyEditor
            v-model="selected!"
            :saving="saving"
            :duplicating="duplicating"
            @save="saveStrategy"
            @cancel="cancelEdit"
          />
        </div>
      </Transition>
    </div>
  </div>
</template>

<style scoped>
.full-page-slide-enter-active,
.full-page-slide-leave-active {
  transition: all 0.35s cubic-bezier(0.4, 0, 0.2, 1);
}
.full-page-slide-enter-from {
  transform: translateX(100%);
  opacity: 0;
}
.full-page-slide-leave-to {
  transform: translateX(-100%);
  opacity: 0;
}
</style>
