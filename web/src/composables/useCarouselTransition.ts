import { ref, watch, type Ref } from "vue"

export type SlideDirection = "forward" | "backward"

/**
 * Tracks a numeric step value and derives the slide direction
 * (forward when the step increases, backward when it decreases).
 *
 * Pass the returned `direction` to `<SlideTransition :direction="direction">`.
 *
 * @example
 * const step = computed(() => {
 *   if (isEditing.value) return 2
 *   if (selected.value) return 1
 *   return 0
 * })
 * const { direction } = useCarouselTransition(step)
 */
export function useCarouselTransition(step: Ref<number>) {
  const direction = ref<SlideDirection>("forward")

  watch(step, (newStep, oldStep) => {
    direction.value = newStep >= (oldStep ?? newStep) ? "forward" : "backward"
  })

  return { direction }
}
