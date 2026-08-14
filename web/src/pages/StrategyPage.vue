<script setup lang="ts">
import { computed } from "vue"
import Button from "primevue/button"
import PageHeader from "@/components/layout/PageHeader.vue"
import SlideTransition from "@/components/SlideTransition.vue"
import StrategyEditor from "@/components/strategy/StrategyEditor.vue"
import StrategyDetail from "@/components/strategy/StrategyDetail.vue"
import StrategyList from "@/components/strategy/StrategyList.vue"
import StrategyPromptPreview from "@/components/strategy/StrategyPromptPreview.vue"
import StrategyTestResult from "@/components/strategy/StrategyTestResult.vue"
import { useStrategyPage } from "@/composables/useStrategyPage"
import { useCarouselTransition } from "@/composables/useCarouselTransition"
import type { EditableStrategy } from "@/types/strategy-ui"

const {
  createNew,
  deleteStrategy,
  duplicateStrategy,
  duplicating,
  loading,
  positions,
  positionsLoading,
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

// View indices for carousel direction tracking: 0 = list, 1 = detail, 2 = editor
const step = computed(() => {
  if (isEditing.value) return 2
  if (selected.value) return 1
  return 0
})

const { direction } = useCarouselTransition(step)

// The editor is only rendered while `isEditing`, which implies a non-null selection.
const editingStrategy = computed<EditableStrategy>({
  get: () => selected.value as EditableStrategy,
  set: (value) => {
    selected.value = value
  },
})
</script>

<template>
  <div class="flex flex-col gap-6 w-full">
    <PageHeader
      title="Strategy Studio"
      description="Create and manage trading strategies"
    >
      <template #actions>
        <Button
          label="New Strategy"
          icon="pi pi-plus"
          class="rounded-xl h-11 px-4 cursor-pointer"
          @click="createNew"
        />
      </template>
    </PageHeader>

    <div class="relative w-full overflow-x-clip min-h-125">
      <SlideTransition :direction="direction">
        <!-- View 1: Strategy List Screen (Full Screen Grid) -->
        <div v-if="step === 0" class="w-full" key="list-view">
          <StrategyList
            :strategies="strategies"
            :loading="loading"
            @select="selectStrategy"
          />
        </div>

        <!-- View 2: Strategy Detail Screen (Full Screen Detail Panel with Back Button) -->
        <div
          v-else-if="step === 1"
          class="flex flex-col gap-4 w-full"
          key="detail-view"
        >
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
            :strategy="selected!"
            :duplicating="duplicating"
            :test-run-loading="testRunLoading"
            :preview-loading="previewLoading"
            :positions="positions"
            :positions-loading="positionsLoading"
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
        <div v-else-if="step === 2" class="w-full" key="edit-view">
          <StrategyEditor
            v-model="editingStrategy"
            :saving="saving"
            :duplicating="duplicating"
            :test-run-loading="testRunLoading"
            @save="saveStrategy"
            @cancel="cancelEdit"
            @test="runTest"
          />
          <StrategyTestResult v-if="testResult" :result="testResult" />
        </div>
      </SlideTransition>
    </div>
  </div>
</template>
