import type { AppError } from '@/api/types'

export function normalizeError(error: unknown): AppError {
  if (typeof error === 'string') return { kind: 'Unknown', message: error }
  if (error && typeof error === 'object') {
    const record = error as Record<string, unknown>
    if (typeof record.kind === 'string' && typeof record.message === 'string') {
      return { kind: record.kind, message: record.message }
    }
    if (typeof record.message === 'string') {
      return { kind: 'Unknown', message: record.message }
    }
  }
  return { kind: 'Unknown', message: String(error) }
}

export function errorText(error: unknown): string {
  return normalizeError(error).message
}
