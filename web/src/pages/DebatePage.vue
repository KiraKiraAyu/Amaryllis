<script setup lang="ts">
import { computed } from "vue"
import Button from "primevue/button"
import CreateDebateModal from "@/components/debate/CreateDebateModal.vue"
import DebateDetail from "@/components/debate/DebateDetail.vue"
import DebateSessionsList from "@/components/debate/DebateSessionsList.vue"
import PageHeader from "@/components/layout/PageHeader.vue"
import SlideTransition from "@/components/SlideTransition.vue"
import { useDebatePage } from "@/composables/useDebatePage"
import { useCarouselTransition } from "@/composables/useCarouselTransition"

const {
  activeDebate,
  cancelDebate,
  createDebate,
  creatingDebate,
  debates,
  loadingDebates,
  messages,
  newDebate,
  personalities,
  personalityEmoji,
  selectDebate,
  showCreate,
  startDebate,
  togglePersonality,
} = useDebatePage()

// 0 = empty placeholder, 1 = debate detail
const step = computed(() => (activeDebate.value ? 1 : 0))
const { direction } = useCarouselTransition(step)
</script>

<template>
  <div class="flex flex-col gap-6">
    <PageHeader
      title="AI Debate Arena"
      description="Multiple AI personalities debate trading decisions"
    >
      <template #actions>
        <Button @click="showCreate = true" icon="pi pi-plus" label="New Debate" class="rounded-xl h-11 px-4 cursor-pointer" />
      </template>
    </PageHeader>

    <div class="grid grid-cols-1 gap-6 lg:grid-cols-3">
      <DebateSessionsList
        :debates="debates"
        :active-id="activeDebate?.id"
        :loading="loadingDebates"
        @select="selectDebate"
      />

      <div class="lg:col-span-2 relative overflow-hidden">
        <SlideTransition :direction="direction">
          <div v-if="step === 0" key="empty" class="flex items-center justify-center h-64">
            <p class="text-sm text-text-muted">
              Select a debate session to view
            </p>
          </div>

          <DebateDetail
            v-else
            key="detail"
            :debate="activeDebate!"
            :messages="messages"
            @start="startDebate"
            @cancel="cancelDebate"
          />
        </SlideTransition>
      </div>
    </div>
  </div>

  <CreateDebateModal
    v-if="showCreate"
    v-model="newDebate"
    :personalities="personalities"
    :creating="creatingDebate"
    :personality-emoji="personalityEmoji"
    @create="createDebate"
    @close="showCreate = false"
    @toggle-personality="togglePersonality"
  />
</template>
