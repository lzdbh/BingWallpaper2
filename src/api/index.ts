import { isTauri } from './events'
import { realApi } from './commands'
import { mockApi } from './mock'

export const api = isTauri ? realApi : mockApi
export { isTauri }
export * from './types'
