import { listen, type UnlistenFn } from '@tauri-apps/api/event'

export const isTauri =
  typeof window !== 'undefined' && Boolean(window.__TAURI_INTERNALS__)

type Handler = (payload: unknown) => void

class MockEventBus {
  private handlers = new Map<string, Set<Handler>>()

  on(event: string, handler: Handler): UnlistenFn {
    const set = this.handlers.get(event) ?? new Set<Handler>()
    set.add(handler)
    this.handlers.set(event, set)
    return () => {
      set.delete(handler)
      if (set.size === 0) this.handlers.delete(event)
    }
  }

  emit(event: string, payload: unknown) {
    for (const handler of this.handlers.get(event) ?? []) {
      try {
        handler(payload)
      } catch (error) {
        console.error(`[mock-event] ${event}`, error)
      }
    }
  }

  clear() {
    this.handlers.clear()
  }
}

export const mockBus = new MockEventBus()

/** Subscribe to a backend event. Falls back to the in-browser mock bus. */
export async function onEvent<T>(
  event: string,
  handler: (payload: T) => void,
): Promise<UnlistenFn> {
  if (isTauri) {
    return listen<T>(event, (e) => handler(e.payload))
  }
  return mockBus.on(event, handler as Handler)
}

export async function emitLocal(event: string, payload: unknown) {
  if (!isTauri) mockBus.emit(event, payload)
}
