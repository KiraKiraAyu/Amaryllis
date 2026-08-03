<script setup lang="ts">
/**
 * A reusable carousel-style slide transition.
 *
 * Unlike `mode="out-in"` transitions where the old view fully exits
 * before the new one enters, this component shows both views
 * simultaneously — the leaving view slides out while the entering
 * view slides in, producing a carousel / swiper effect.
 *
 * The leaving view is absolutely positioned during the transition so
 * the entering view takes its natural place in the layout.
 *
 * @prop direction - "forward" (default) slides left-to-right → new from right,
 *                   "backward" slides right-to-left → new from left.
 *
 * @example
 * <SlideTransition :direction="direction">
 *   <div :key="step" class="w-full">...</div>
 * </SlideTransition>
 */
import { computed } from "vue"
import type { SlideDirection } from "@/composables/useCarouselTransition"

const props = withDefaults(
  defineProps<{
    direction?: SlideDirection
  }>(),
  {
    direction: "forward",
  },
)

const transitionName = computed(() =>
  props.direction === "backward" ? "carousel-backward" : "carousel-forward",
)
</script>

<template>
  <Transition :name="transitionName">
    <slot />
  </Transition>
</template>

<style>
/* Non-scoped: transition classes are applied to slot content
   which originates from the parent, so scoped data-attributes
   won't match. Class names are prefixed to avoid collisions. */

.carousel-forward-enter-active,
.carousel-forward-leave-active,
.carousel-backward-enter-active,
.carousel-backward-leave-active {
  transition: transform 0.4s cubic-bezier(0.4, 0, 0.2, 1);
  will-change: transform;
}

/* Leaving view overlays so entering view takes layout space */
.carousel-forward-leave-active,
.carousel-backward-leave-active {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
}

/* Forward: new enters from right, old exits to left */
.carousel-forward-enter-from {
  transform: translateX(100%);
}
.carousel-forward-leave-to {
  transform: translateX(-100%);
}

/* Backward: new enters from left, old exits to right */
.carousel-backward-enter-from {
  transform: translateX(-100%);
}
.carousel-backward-leave-to {
  transform: translateX(100%);
}
</style>
