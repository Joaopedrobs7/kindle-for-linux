export interface ReaderPrefs {
  theme: string
  fontFamily: string
  fontWeight: number
  fontSize: number
  lineHeight: number
  measure: number
  align: string
  flow: string
}

export interface Settings {
  appearance: string
  region: string
  reader: ReaderPrefs
}

export interface Book {
  id: string
  source: string
  asin: string
  title: string
  author: string
  filePath: string
  cover: string
  percentage: number
  chapterTitle: string
  missing: boolean
  addedAt: number
}

export interface Highlight {
  id: string
  bookId: string
  bookTitle: string
  author: string
  source: string
  cfi: string
  text: string
  note: string
  color: string
  startPos: number
  endPos: number
  createdAt: number
}

export interface Bookmark {
  id: string
  bookId: string
  cfi: string
  title: string
  createdAt: number
}

export interface SessionStatus {
  signedIn: boolean
  region: string
  cookieNames: string[]
  hasUhura: boolean
  hasDeviceToken: boolean
}

export interface Snapshot {
  books: Book[]
  highlights: Highlight[]
  bookmarks: Bookmark[]
  settings: Settings
  session: SessionStatus
  folders: string[]
}

export interface Definition {
  word: string
  partOfSpeech: string
  senses: string[]
  wikipediaTitle: string
  wikipediaExtract: string
  wikipediaUrl: string
  source: string
}

export interface NameHit {
  name: string
  count: number
}

export interface Overview {
  words: number
  names: NameHit[]
  percentage: number
  minutesLeft: number
}

export interface TocEntry {
  label: string
  href: string
  subitems?: TocEntry[]
}

export const EMPTY_SESSION: SessionStatus = {
  signedIn: false,
  region: 'br',
  cookieNames: [],
  hasUhura: false,
  hasDeviceToken: false,
}
