import { invoke } from '@tauri-apps/api/core'
import { mock } from '@/lib/mock'
import type { Bookmark, Definition, Highlight, Overview, ReaderPrefs, Settings, Snapshot } from '@/types'

export function inTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

export async function loadSnapshot(): Promise<Snapshot> {
  if (!inTauri()) return mock.snapshot()
  return invoke<Snapshot>('library_snapshot')
}

export async function readBook(bookId: string): Promise<Uint8Array> {
  if (!inTauri()) return new Uint8Array(await mock.readBook(bookId))
  const bytes = await invoke<number[]>('read_book', { bookId })
  return new Uint8Array(bytes)
}

export async function bookCfi(bookId: string): Promise<string> {
  if (!inTauri()) return ''
  return invoke<string>('book_cfi', { bookId })
}

export async function saveProgress(input: { bookId: string; cfi: string; percentage: number; chapterTitle?: string }) {
  if (!inTauri()) return mock.saveProgress(input)
  await invoke('save_progress', { input })
}

export async function addHighlight(input: { bookId: string; cfi: string; text: string; note?: string; color?: string }): Promise<Highlight> {
  if (!inTauri()) return mock.addHighlight(input)
  return invoke<Highlight>('add_highlight', { input })
}

export async function updateHighlight(id: string, note: string) {
  if (!inTauri()) return mock.updateHighlight(id, note)
  await invoke('update_highlight', { id, note })
}

export async function removeHighlight(id: string) {
  if (!inTauri()) return mock.removeHighlight(id)
  await invoke('remove_highlight', { id })
}

export async function addBookmark(input: { bookId: string; cfi: string; title: string }) {
  if (!inTauri()) return mock.addBookmark(input)
  await invoke('add_bookmark', { input })
}

export async function removeBookmark(id: string) {
  if (!inTauri()) return mock.removeBookmark(id)
  await invoke('remove_bookmark', { id })
}

export async function removeBook(id: string) {
  if (!inTauri()) return mock.removeBook(id)
  return invoke<Snapshot>('remove_book', { id })
}

export async function saveSettings(settings: Settings) {
  if (!inTauri()) return mock.saveSettings(settings)
  return invoke<Snapshot>('save_settings', { settings })
}

export async function exportClippings(format: 'markdown' | 'clippings') {
  if (!inTauri()) return mock.exportClippings(format)
  return invoke<string>('export_clippings', { format })
}

export async function defineWord(word: string): Promise<Definition> {
  if (!inTauri()) return mock.defineWord(word)
  return invoke<Definition>('define_word', { word })
}

export async function bookOverview(bookId: string, wordsPerMinute?: number): Promise<Overview> {
  if (!inTauri()) return mock.bookOverview(bookId)
  return invoke<Overview>('book_overview', { bookId, wordsPerMinute })
}

export async function importEpub(path: string) {
  if (!inTauri()) return mock.snapshot()
  return invoke<Snapshot>('import_epub', { path })
}

export async function cloudOpenLogin(region: string) {
  if (!inTauri()) return
  await invoke('cloud_open_login', { region })
}

export async function cloudCapture() {
  if (!inTauri()) return mock.snapshot()
  await invoke('cloud_capture')
  return invoke<Snapshot>('library_snapshot')
}

export async function cloudImportCookies(header: string, region: string) {
  if (!inTauri()) return mock.cloudImport(header, region)
  await invoke('cloud_import_cookies', { header, region })
  return invoke<Snapshot>('library_snapshot')
}

export async function cloudSync(region?: string) {
  if (!inTauri()) return mock.cloudLoadExample()
  return invoke<Snapshot>('cloud_sync', { region })
}

export async function cloudSyncBook(asin: string) {
  if (!inTauri()) return mock.snapshot()
  return invoke<Snapshot>('cloud_sync_book', { asin })
}

export async function cloudLoadExample() {
  if (!inTauri()) return mock.cloudLoadExample()
  return invoke<Snapshot>('cloud_load_example')
}

export async function cloudOpenReader(asin: string, title?: string) {
  if (!inTauri()) return
  await invoke('cloud_open_reader', { asin, title })
}

export async function cloudCloseReader() {
  if (!inTauri()) return
  await invoke('cloud_close_reader')
}

export async function setFullscreen(enabled: boolean) {
  if (!inTauri()) {
    if (enabled) await document.documentElement.requestFullscreen?.()
    else if (document.fullscreenElement) await document.exitFullscreen?.()
    return
  }
  await invoke('set_fullscreen', { enabled })
}

export function patchReader(settings: Settings, reader: Partial<ReaderPrefs>): Settings {
  return { ...settings, reader: { ...settings.reader, ...reader } }
}

export type { Bookmark }
