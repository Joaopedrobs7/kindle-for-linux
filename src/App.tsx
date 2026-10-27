import { useCallback, useEffect, useState } from 'react'
import {
  addBookmark,
  addHighlight,
  bookCfi,
  cloudCloseReader,
  cloudLoadExample,
  cloudOpenLogin,
  cloudOpenReader,
  cloudSync,
  cloudSyncBook,
  exportClippings,
  importEpub,
  inTauri,
  loadSnapshot,
  patchReader,
  readBook,
  removeBook,
  removeBookmark,
  removeHighlight,
  saveProgress,
  saveSettings,
  setFullscreen,
  updateHighlight,
} from '@/lib/api'
import { Sidebar, type View } from '@/components/Sidebar'
import type { Book, ReaderPrefs, Snapshot } from '@/types'
import { Cloud } from '@/views/Cloud'
import { Notebook } from '@/views/Notebook'
import { Reader } from '@/views/Reader'
import { SettingsView } from '@/views/Settings'
import { Shelf } from '@/views/Shelf'

export function App() {
  const [snap, setSnap] = useState<Snapshot | null>(null)
  const [view, setView] = useState<View>('shelf')
  const [reading, setReading] = useState<Book | null>(null)
  const [bytes, setBytes] = useState<Uint8Array | null>(null)
  const [startCfi, setStartCfi] = useState('')
  const [loadError, setLoadError] = useState('')
  const [query, setQuery] = useState('')
  const [message, setMessage] = useState('')
  const [busy, setBusy] = useState(false)
  const [command, setCommand] = useState<{ id: number; name: string } | null>(null)
  const [full, setFull] = useState(false)

  const refresh = useCallback(async () => {
    setSnap(await loadSnapshot())
  }, [])

  useEffect(() => {
    let stops: Array<() => void> = []
    let cancel = false
    void (async () => {
      try {
        const next = await loadSnapshot()
        if (cancel) return
        setSnap(next)
        if (inTauri() && next.session.signedIn) {
          try {
            const synced = await cloudSync(next.settings.region)
            if (!cancel) setSnap(synced)
          } catch {
            /* a estante local continua visível */
          }
        }
      } catch (error: unknown) {
        if (!cancel) setMessage(error instanceof Error ? error.message : 'Falha ao abrir a estante')
      }
    })()
    if (inTauri()) {
      void import('@tauri-apps/api/event')
        .then(({ listen }) =>
          Promise.all([
            listen('library-changed', () => {
              void refresh()
            }),
            listen('cloud-reader-closed', () => {
              setReading((current) => (current?.source === 'cloud' ? null : current))
              setView('shelf')
              void refresh()
            }),
          ]),
        )
        .then((unlisten) => {
          if (cancel) unlisten.forEach((stop) => stop())
          else stops = unlisten
        })
        .catch(() => undefined)
    }
    return () => {
      cancel = true
      stops.forEach((stop) => stop())
    }
  }, [refresh])

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const meta = event.ctrlKey || event.metaKey
      if (event.key === 'F11') {
        event.preventDefault()
        setFull((value) => {
          void setFullscreen(!value)
          return !value
        })
      } else if (event.key === 'Escape') {
        if (reading?.source === 'cloud') {
          event.preventDefault()
          setReading(null)
          setView('shelf')
          void cloudCloseReader()
          void refresh()
        } else {
          setCommand({ id: Date.now(), name: 'escape' })
        }
      } else if (meta && event.key.toLowerCase() === 'f' && reading) {
        event.preventDefault()
        setCommand({ id: Date.now(), name: 'search' })
      } else if (meta && event.key.toLowerCase() === 'b' && reading) {
        event.preventDefault()
        setCommand({ id: Date.now(), name: 'bookmark' })
      } else if (meta && (event.key === '+' || event.key === '=')) {
        event.preventDefault()
        setCommand({ id: Date.now(), name: 'zoom-in' })
      } else if (meta && event.key === '-') {
        event.preventDefault()
        setCommand({ id: Date.now(), name: 'zoom-out' })
      } else if (meta && event.key === '1') {
        if (reading?.source === 'cloud') void cloudCloseReader()
        setReading(null)
        setView('shelf')
      } else if (meta && event.key === '2') {
        if (reading?.source === 'cloud') void cloudCloseReader()
        setReading(null)
        setView('cloud')
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [reading])

  function closeCloudReader() {
    setReading(null)
    setView('shelf')
    void cloudCloseReader()
    void refresh()
  }

  async function openCloudBook(book: Book) {
    setReading(book)
    setView('shelf')
    try {
      await cloudOpenReader(book.asin, book.title)
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'Não abriu o leitor da Amazon')
    }
  }

  async function openBook(book: Book) {
    if (book.source === 'cloud') {
      await openCloudBook(book)
      return
    }
    setReading(book)
    setBytes(null)
    setLoadError('')
    try {
      const [next, cfi] = await Promise.all([readBook(book.id), bookCfi(book.id).catch(() => '')])
      setBytes(next)
      setStartCfi(cfi)
    } catch (error) {
      setLoadError(error instanceof Error ? error.message : 'Não foi possível abrir o EPUB')
    }
  }

  async function persistReader(patch: Partial<ReaderPrefs>) {
    if (!snap) return
    const next = await saveSettings(patchReader(snap.settings, patch))
    setSnap(next)
  }

  function download(filename: string, contents: string) {
    const blob = new Blob([contents], { type: 'text/plain;charset=utf-8' })
    const url = URL.createObjectURL(blob)
    const link = document.createElement('a')
    link.href = url
    link.download = filename
    link.click()
    URL.revokeObjectURL(url)
  }

  if (!snap) {
    return <div className="boot">Abrindo a estante…</div>
  }

  const appearance = snap.settings.appearance === 'ink' ? 'ink' : 'paper'

  return (
    <div className="app" data-appearance={appearance} data-fullscreen={full ? 'yes' : 'no'}>
      {!reading && <Sidebar view={view} onNavigate={(next) => { setView(next); setMessage('') }} />}
      <main>
        {reading?.source === 'cloud' ? (
          <div className="cloud-read" data-testid="cloud-reader">
            <header className="reader-bar">
              <button type="button" onClick={closeCloudReader}>← Biblioteca</button>
              <span aria-hidden="true">·</span>
              <strong className="reader-bar-title">{reading.title}</strong>
              <span className="reader-bar-author">{reading.author}</span>
              <span className="reader-bar-spacer" />
              <button
                type="button"
                onClick={() => setFull((value) => {
                  void setFullscreen(!value)
                  return !value
                })}
              >
                Tela cheia
              </button>
            </header>
            <p className="cloud-read-wait">Abrindo o livro…</p>
          </div>
        ) : reading ? (
          <Reader
            key={reading.id}
            bookId={reading.id}
            title={reading.title}
            author={reading.author}
            data={bytes}
            startCfi={startCfi}
            loadError={loadError}
            prefs={snap.settings.reader}
            bookmarks={snap.bookmarks.filter((item) => item.bookId === reading.id)}
            command={command}
            onBack={() => { setReading(null); void refresh() }}
            onPrefs={(patch) => void persistReader(patch)}
            onProgress={(input) => void saveProgress({ bookId: reading.id, ...input }).then(() => refresh())}
            onHighlight={(input) => void addHighlight({ bookId: reading.id, ...input }).then(() => refresh())}
            onBookmark={(input) => void addBookmark({ bookId: reading.id, ...input }).then(() => refresh())}
            onRemoveBookmark={(id) => void removeBookmark(id).then(() => refresh())}
          />
        ) : view === 'shelf' ? (
          <Shelf
            books={snap.books}
            query={query}
            onQuery={setQuery}
            onOpen={(book) => void openBook(book)}
            onRemove={(book) => void removeBook(book.id).then(setSnap)}
            signedIn={snap.session.signedIn}
            onLogin={() => void cloudOpenLogin(snap.settings.region).catch((error: unknown) => setMessage(error instanceof Error ? error.message : 'Abra pelo aplicativo de desktop para entrar na Amazon'))}
            onImport={() => setView('settings')}
          />
        ) : view === 'notebook' ? (
          <Notebook
            highlights={snap.highlights}
            onNote={(id, note) => void updateHighlight(id, note).then(() => refresh())}
            onRemove={(id) => void removeHighlight(id).then(() => refresh())}
            onExport={(format) => void exportClippings(format).then((text) => download(format === 'clippings' ? 'My Clippings.txt' : 'caderno.md', text))}
          />
        ) : view === 'cloud' ? (
          <Cloud
            books={snap.books}
            session={snap.session}
            region={snap.settings.region}
            busy={busy}
            message={message}
            onRegion={(region) => void saveSettings({ ...snap.settings, region }).then(setSnap)}
            onLogin={() => void cloudOpenLogin(snap.settings.region).catch((error: unknown) => setMessage(error instanceof Error ? error.message : 'Abra pelo aplicativo de desktop para entrar na Amazon'))}
            onSync={() => {
              setBusy(true)
              void cloudSync(snap.settings.region).then(setSnap).catch((error: unknown) => setMessage(error instanceof Error ? error.message : 'Falha ao sincronizar')).finally(() => setBusy(false))
            }}
            onExample={() => void cloudLoadExample().then((next) => { setSnap(next); setMessage('Exemplo offline da Kindle Cloud, sem credenciais.') })}
            onOpen={(asin) => {
              const book = snap.books.find((item) => item.asin === asin) ?? {
                id: `cloud:${asin}`,
                source: 'cloud',
                asin,
                title: asin,
                author: '',
                filePath: '',
                cover: '',
                percentage: 0,
                chapterTitle: '',
                missing: false,
                addedAt: 0,
              }
              void openCloudBook(book)
            }}
            onAnnotations={(asin) => {
              setBusy(true)
              void cloudSyncBook(asin).then(setSnap).catch((error: unknown) => setMessage(error instanceof Error ? error.message : 'Sem anotações')).finally(() => setBusy(false))
            }}
          />
        ) : (
          <SettingsView
            settings={snap.settings}
            folders={snap.folders}
            onChange={(next) => void saveSettings(next).then(setSnap)}
            onImport={(path) => void importEpub(path).then(setSnap).catch((error: unknown) => setMessage(error instanceof Error ? error.message : 'Falha ao importar'))}
          />
        )}
      </main>
    </div>
  )
}
