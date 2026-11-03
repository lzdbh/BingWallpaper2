import { describe, expect, it } from 'vitest'
import { errorText, normalizeError } from './error'

describe('normalizeError', () => {
  it('wraps a plain string as an Unknown error', () => {
    expect(normalizeError('boom')).toEqual({ kind: 'Unknown', message: 'boom' })
  })

  it('keeps a backend AppError shape ({ kind, message })', () => {
    const error = { kind: 'Network', message: '请求超时' }
    expect(normalizeError(error)).toEqual({ kind: 'Network', message: '请求超时' })
  })

  it('falls back to Unknown when only a message is present', () => {
    expect(normalizeError({ message: '只有消息' })).toEqual({
      kind: 'Unknown',
      message: '只有消息',
    })
  })

  it('handles a native Error instance', () => {
    const result = normalizeError(new Error('native'))
    expect(result).toEqual({ kind: 'Unknown', message: 'native' })
  })

  it('stringifies null/undefined/other values safely', () => {
    expect(normalizeError(null).message).toBe('null')
    expect(normalizeError(undefined).message).toBe('undefined')
    expect(normalizeError(42)).toEqual({ kind: 'Unknown', message: '42' })
  })

  it('ignores a non-string kind field', () => {
    // kind must be a string to be trusted; otherwise fall back to Unknown.
    expect(normalizeError({ kind: 123, message: 'x' })).toEqual({
      kind: 'Unknown',
      message: 'x',
    })
  })
})

describe('errorText', () => {
  it('returns just the message', () => {
    expect(errorText({ kind: 'Io', message: '磁盘错误' })).toBe('磁盘错误')
    expect(errorText('直接字符串')).toBe('直接字符串')
  })
})
