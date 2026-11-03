import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from '@tauri-apps/plugin-notification'
import { isTauri } from '@/api/events'

export async function notify(title: string, body: string) {
  if (!isTauri) return
  try {
    let granted = await isPermissionGranted()
    if (!granted) {
      granted = (await requestPermission()) === 'granted'
    }
    if (granted) sendNotification({ title, body })
  } catch (error) {
    console.warn('[notification]', error)
  }
}
