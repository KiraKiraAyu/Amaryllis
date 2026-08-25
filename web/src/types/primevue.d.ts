declare module "primevue/toasteventbus" {
  export interface ToastEventBusType {
    on(type: string, handler: (data: unknown) => void): void
    off(type: string, handler: (data: unknown) => void): void
    emit(type: string, data?: unknown): void
  }

  const ToastEventBus: ToastEventBusType
  export default ToastEventBus
}
