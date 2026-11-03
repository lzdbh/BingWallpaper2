import { onBeforeUnmount, onMounted } from 'vue'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { onEvent } from '@/api/events'

export function useTauriEvent<T>(event: string, handler: (payload: T) => void) {
  let unlisten: UnlistenFn | undefined
  let disposed = false

  onMounted(async () => {
    const off = await onEvent<T>(event, handler)
    if (disposed) off()
    else unlisten = off
  })

  onBeforeUnmount(() => {
    disposed = true
    unlisten?.()
  })
}
