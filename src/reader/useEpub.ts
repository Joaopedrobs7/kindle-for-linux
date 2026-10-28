import { useCallback, useEffect, useRef, useState } from 'react'
import ePub, { type Book, type Contents, type Location, type Rendition } from 'epubjs'
import { fontStack, themeOf, themeRules } from '@/lib/themes'
import type { ReaderPrefs, TocEntry } from '@/types'

export interface SelectionInfo {
  cfi: string
  text: string
}

export interface SearchHit {
  cfi: string
  excerpt: string
}

interface SearchableSection {
  href: string
  load: (request: unknown) => Promise<unknown>
  search?: (query: string) => Array<{ cfi: string; excerpt: string }>
  unload: () => void
}

export function useEpub({
  bookId,
  data,
  initialCfi,
  prefs,
}: {
  bookId: string
  data: Uint8Array | null
  initialCfi: string
  prefs: ReaderPrefs
}) {
  const hostRef = useRef<HTMLDivElement>(null)
  const renditionRef = useRef<Rendition | null>(null)
  const bookRef = useRef<Book | null>(null)
  const prefsRef = useRef(prefs)
  const cfiRef = useRef(initialCfi)
  const hrefRef = useRef('')
  prefsRef.current = prefs

  const [status, setStatus] = useState<'loading' | 'ready' | 'error'>('loading')
  const [error, setError] = useState('')
  const [toc, setToc] = useState<TocEntry[]>([])
  const [percentage, setPercentage] = useState(0)
  const [chapterTitle, setChapterTitle] = useState('')
  const [pageLabel, setPageLabel] = useState('')
  const [selection, setSelection] = useState<SelectionInfo | null>(null)

  useEffect(() => {
    const host = hostRef.current
    if (!host || !data || !bookId) return
    let stopped = false
    let started = false
    let resizeObserver: ResizeObserver | null = null
    const kick = new ResizeObserver(() => void start())
    kick.observe(host)

    const start = async () => {
      if (stopped || started || host.clientWidth < 40 || host.clientHeight < 40) return
      started = true
      setStatus('loading')
      try {
        const book = ePub(toArrayBuffer(data))
        bookRef.current = book
        await book.ready
        if (stopped) return
        const rendition = book.renderTo(host, {
          width: host.clientWidth,
          height: host.clientHeight,
          flow: prefsRef.current.flow === 'scrolled' ? 'scrolled' : 'paginated',
          spread: prefsRef.current.columns === 1 ? 'none' : 'auto',
          minSpreadWidth: 720,
          manager: prefsRef.current.flow === 'scrolled' ? 'continuous' : 'default',
          allowScriptedContent: false,
        })
        renditionRef.current = rendition
        applyPrefs(rendition, prefsRef.current)
        const captureSelection = (contents: Contents) => {
          const info = selectionFrom(contents)
          if (info) setSelection(info)
        }
        rendition.hooks.content.register((contents: Contents) => {
          paint(contents?.document, prefsRef.current)
          contents?.document?.addEventListener('selectionchange', () => captureSelection(contents))
          contents?.document?.addEventListener('mouseup', () => captureSelection(contents))
        })
        rendition.on('relocated', (location: Location) => {
          if (stopped || !location?.start) return
          cfiRef.current = location.start.cfi
          hrefRef.current = location.start.href
          const total = (book.spine as unknown as { spineItems?: unknown[] }).spineItems?.length ?? 1
          const displayed = location.start.displayed
          const pageFrac = displayed?.total ? Math.max(0, displayed.page - 1) / displayed.total : 0
          const fallback = (location.start.index + pageFrac) / Math.max(1, total)
          const fromLocations = location.start.percentage
          const next = typeof fromLocations === 'number' && fromLocations > 0 ? fromLocations : fallback
          setPercentage(Math.min(1, Math.max(0, next)))
          setChapterTitle(chapterFrom(book, location.start.href))
          setPageLabel(displayed?.total ? `${displayed.page} / ${displayed.total}` : '')
        })
        rendition.on('selected', (cfiRange: string, contents: Contents) => {
          const text = contents?.window?.getSelection?.()?.toString().trim() ?? ''
          if (text && cfiRange) setSelection({ cfi: cfiRange, text: text.slice(0, 800) })
        })
        try {
          if (cfiRef.current) await rendition.display(cfiRef.current)
          else await rendition.display()
        } catch {
          await rendition.display()
        }
        const navigation = await book.loaded.navigation
        if (!stopped) setToc(simplify(navigation.toc))
        setStatus('ready')
        resizeObserver = new ResizeObserver(() => {
          if (host.clientWidth > 40) rendition.resize(host.clientWidth, host.clientHeight)
        })
        resizeObserver.observe(host)
        void book.locations.generate(1600)
      } catch (cause) {
        if (!stopped) {
          setStatus('error')
          setError(cause instanceof Error ? cause.message : 'Não foi possível abrir este EPUB.')
        }
      }
    }
    void start()
    return () => {
      stopped = true
      kick.disconnect()
      resizeObserver?.disconnect()
      try { renditionRef.current?.destroy() } catch { /* epub.js */ }
      try { bookRef.current?.destroy() } catch { /* epub.js */ }
      renditionRef.current = null
      bookRef.current = null
    }
  }, [bookId, data, prefs.flow, prefs.columns])

  useEffect(() => {
    const rendition = renditionRef.current
    const host = hostRef.current
    if (!rendition || status !== 'ready') return
    applyPrefs(rendition, prefs)
    const contents = rendition.getContents()
    if (Array.isArray(contents)) {
      for (const content of contents) paint((content as Contents).document, prefs)
    }
    if (host && host.clientWidth > 40) rendition.resize(host.clientWidth, host.clientHeight)
  }, [prefs, status])

  const next = useCallback(async () => { try { await renditionRef.current?.next() } catch { /* fim */ } }, [])
  const prev = useCallback(async () => { try { await renditionRef.current?.prev() } catch { /* início */ } }, [])
  const goTo = useCallback(async (target: string) => { try { await renditionRef.current?.display(target) } catch { /* href */ } }, [])
  const currentCfi = useCallback(() => cfiRef.current, [])
  const grabSelection = useCallback(() => {
    const contents = renditionRef.current?.getContents()
    if (!Array.isArray(contents)) return null
    for (const content of contents as Contents[]) {
      const info = selectionFrom(content)
      if (info) {
        setSelection(info)
        return info
      }
    }
    return null
  }, [])

  const clearSelection = useCallback(() => {
    const contents = renditionRef.current?.getContents()
    if (Array.isArray(contents)) {
      for (const content of contents) (content as Contents).window?.getSelection?.()?.removeAllRanges()
    }
    setSelection(null)
  }, [])

  const search = useCallback(async (query: string): Promise<SearchHit[]> => {
    const book = bookRef.current
    const needle = query.trim()
    if (!book || needle.length < 2) return []
    const items = (book.spine as unknown as { spineItems?: SearchableSection[] }).spineItems ?? []
    const hits: SearchHit[] = []
    for (const section of items) {
      if (hits.length >= 30) break
      try {
        await section.load(book.load.bind(book))
        for (const hit of section.search?.(needle) ?? []) {
          hits.push({ cfi: hit.cfi, excerpt: hit.excerpt.replace(/<[^>]+>/g, ' ').replace(/\s+/g, ' ').trim() })
          if (hits.length >= 30) break
        }
      } catch { /* seção */ }
      finally {
        if (section.href !== hrefRef.current) {
          try { section.unload() } catch { /* ignore */ }
        }
      }
    }
    return hits
  }, [])

  return { hostRef, status, error, toc, percentage, chapterTitle, pageLabel, selection, next, prev, goTo, currentCfi, clearSelection, grabSelection, search }
}

function selectionFrom(contents: Contents): SelectionInfo | null {
  const selection = contents.window?.getSelection?.()
  const text = selection?.toString().replace(/\s+/g, ' ').trim() ?? ''
  if (!text || !selection || selection.rangeCount === 0) return null
  let cfi = ''
  try {
    cfi = contents.cfiFromRange(selection.getRangeAt(0))
  } catch {
    cfi = ''
  }
  return { cfi, text: text.slice(0, 800) }
}

function applyPrefs(rendition: Rendition, prefs: ReaderPrefs) {
  rendition.themes.register(prefs.theme, themeRules(prefs))
  rendition.themes.select(prefs.theme)
  rendition.themes.fontSize(`${prefs.fontSize}px`)
  rendition.themes.font(fontStack(prefs.fontFamily))
}

function paint(document: Document | undefined, prefs: ReaderPrefs) {
  if (!document?.documentElement) return
  const theme = themeOf(prefs.theme)
  document.documentElement.style.background = theme.bg
  if (document.body) {
    document.body.style.background = theme.bg
    document.body.style.color = theme.fg
    document.body.style.userSelect = 'text'
  }
}

function toArrayBuffer(bytes: Uint8Array): ArrayBuffer {
  const copy = new Uint8Array(bytes.byteLength)
  copy.set(bytes)
  return copy.buffer
}

function chapterFrom(book: Book, href: string): string {
  const items = book.navigation?.toc ?? []
  const found = items.find((item) => href && item.href?.includes(href.split('#')[0]))
  return found?.label?.trim() || ''
}

function simplify(items: Array<{ label: string; href: string; subitems?: Array<{ label: string; href: string }> }> | undefined): TocEntry[] {
  return (items ?? []).map((item) => ({
    label: item.label?.trim() || 'Capítulo',
    href: item.href,
    subitems: item.subitems ? simplify(item.subitems) : undefined,
  }))
}
