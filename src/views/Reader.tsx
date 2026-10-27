import { useEffect, useRef, useState } from 'react'
import { bookOverview, defineWord } from '@/lib/api'
import { FONTS, HIGHLIGHTS, THEMES, themeOf } from '@/lib/themes'
import { useEpub, type SearchHit, type SelectionInfo } from '@/reader/useEpub'
import type { Bookmark, Definition, Overview, ReaderPrefs } from '@/types'

export function Reader({
  bookId,
  title,
  author,
  data,
  startCfi,
  loadError,
  prefs,
  bookmarks,
  command,
  onBack,
  onPrefs,
  onProgress,
  onHighlight,
  onBookmark,
  onRemoveBookmark,
}: {
  bookId: string
  title: string
  author: string
  data: Uint8Array | null
  startCfi: string
  loadError: string
  prefs: ReaderPrefs
  bookmarks: Bookmark[]
  command: { id: number; name: string } | null
  onBack: () => void
  onPrefs: (patch: Partial<ReaderPrefs>) => void
  onProgress: (input: { cfi: string; percentage: number; chapterTitle?: string }) => void
  onHighlight: (input: { cfi: string; text: string; note?: string; color?: string }) => void
  onBookmark: (input: { cfi: string; title: string }) => void
  onRemoveBookmark: (id: string) => void
}) {
  const reader = useEpub({ bookId, data, initialCfi: startCfi, prefs })
  const theme = themeOf(prefs.theme)
  const [panel, setPanel] = useState<'toc' | 'look' | 'xray' | 'dict' | null>(null)
  const [query, setQuery] = useState('')
  const [hits, setHits] = useState<SearchHit[]>([])
  const [searching, setSearching] = useState(false)
  const [color, setColor] = useState('amber')
  const [note, setNote] = useState('')
  const [dictQuery, setDictQuery] = useState('')
  const [definition, setDefinition] = useState<Definition | null>(null)
  const [defining, setDefining] = useState(false)
  const [dictMessage, setDictMessage] = useState('')
  const pendingRef = useRef<SelectionInfo | null>(null)
  const [overview, setOverview] = useState<Overview | null>(null)
  const [seconds, setSeconds] = useState(0)
  const [turns, setTurns] = useState(0)
  const readerRef = useRef(reader)
  const onProgressRef = useRef(onProgress)
  const onBookmarkRef = useRef(onBookmark)
  const onPrefsRef = useRef(onPrefs)
  const prefsRef = useRef(prefs)
  readerRef.current = reader
  onProgressRef.current = onProgress
  onBookmarkRef.current = onBookmark
  onPrefsRef.current = onPrefs
  prefsRef.current = prefs

  useEffect(() => {
    const timer = window.setInterval(() => setSeconds((value) => value + 1), 1000)
    return () => window.clearInterval(timer)
  }, [])

  useEffect(() => {
    if (reader.status !== 'ready') return
    const timer = window.setTimeout(() => {
      const cfi = reader.currentCfi()
      if (cfi) onProgressRef.current({ cfi, percentage: reader.percentage, chapterTitle: reader.chapterTitle })
    }, 280)
    return () => window.clearTimeout(timer)
  }, [reader.status, reader.percentage, reader.chapterTitle, reader.currentCfi])

  useEffect(() => () => {
    const current = readerRef.current
    const cfi = current.currentCfi()
    if (cfi) onProgressRef.current({ cfi, percentage: current.percentage, chapterTitle: current.chapterTitle })
  }, [])

  useEffect(() => {
    if (!command) return
    if (command.name === 'search') setPanel('toc')
    if (command.name === 'bookmark') {
      const cfi = readerRef.current.currentCfi()
      if (cfi) onBookmarkRef.current({ cfi, title: readerRef.current.chapterTitle || 'Marcador' })
    }
    if (command.name === 'zoom-in') onPrefsRef.current({ fontSize: Math.min(32, prefsRef.current.fontSize + 1) })
    if (command.name === 'zoom-out') onPrefsRef.current({ fontSize: Math.max(14, prefsRef.current.fontSize - 1) })
    if (command.name === 'escape') {
      setPanel(null)
      setDefinition(null)
      pendingRef.current = null
      readerRef.current.clearSelection()
    }
  }, [command])

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null
      if (target?.closest('input, textarea, select')) return
      if (event.key === 'ArrowRight' || event.key === 'PageDown' || (event.key === ' ' && !event.shiftKey)) {
        event.preventDefault()
        setTurns((value) => value + 1)
        void reader.next()
      } else if (event.key === 'ArrowLeft' || event.key === 'PageUp' || (event.key === ' ' && event.shiftKey)) {
        event.preventDefault()
        void reader.prev()
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [reader.next, reader.prev])

  useEffect(() => {
    if (panel !== 'xray') return
    const pace = seconds > 20 && turns > 0 ? Math.round((turns * 250) / (seconds / 60)) : 220
    void bookOverview(bookId, pace).then(setOverview).catch(() => setOverview(null))
  }, [panel, bookId, seconds, turns])

  const minutes = Math.floor(seconds / 60)
  const rest = String(seconds % 60).padStart(2, '0')
  const pace = seconds > 20 && turns > 0 ? Math.round((turns * 250) / (seconds / 60)) : null

  async function runSearch(value: string) {
    setQuery(value)
    if (value.trim().length < 2) {
      setHits([])
      return
    }
    setSearching(true)
    try { setHits(await reader.search(value)) } finally { setSearching(false) }
  }

  function firstWord(value: string) {
    return value.trim().split(/\s+/)[0]?.replace(/^[^\p{L}\p{N}]+|[^\p{L}\p{N}]+$/gu, '') ?? ''
  }

  async function runLookup(raw: string) {
    const word = firstWord(raw)
    setDictMessage('')
    if (!word) {
      setDefinition(null)
      return
    }
    setDictQuery(word)
    setDefining(true)
    try { setDefinition(await defineWord(word)) } finally { setDefining(false) }
  }

  function openDictionary(raw?: string) {
    const selected = reader.grabSelection() ?? reader.selection
    if (selected) pendingRef.current = selected
    const word = firstWord(raw || selected?.text || '')
    setPanel('dict')
    if (!word) return
    void runLookup(word)
  }

  async function highlightLookedUp() {
    const word = definition?.word || firstWord(dictQuery)
    if (!word) return
    const selected = reader.selection ?? pendingRef.current
    if (selected?.cfi) {
      onHighlight({ cfi: selected.cfi, text: selected.text, note, color })
      setNote('')
      pendingRef.current = null
      reader.clearSelection()
      setDictMessage('Palavra grifada no texto.')
      return
    }
    const found = (await reader.search(word))[0]
    if (!found) {
      setDictMessage('Esta palavra não aparece no livro aberto.')
      return
    }
    onHighlight({ cfi: found.cfi, text: word, color })
    await reader.goTo(found.cfi)
    setDictMessage('Palavra grifada no texto.')
  }

  function dismissSelection() {
    pendingRef.current = null
    setNote('')
    reader.clearSelection()
  }

  return (
    <section className="reader" data-testid="reader" data-theme={prefs.theme} style={{ background: theme.bg, color: theme.fg }}>
      <header className="reader-chrome top">
        <button type="button" onClick={onBack} aria-label="Voltar à estante">Estante</button>
        <div className="reader-title">
          <strong>{title}</strong>
          <span>{reader.chapterTitle || author}</span>
        </div>
        <div className="reader-actions">
          <button type="button" aria-pressed={panel === 'toc'} onClick={() => setPanel(panel === 'toc' ? null : 'toc')}>Sumário</button>
          <button type="button" aria-label="Aparência da leitura" aria-pressed={panel === 'look'} onClick={() => setPanel(panel === 'look' ? null : 'look')}>Aa</button>
          <button type="button" aria-pressed={panel === 'xray'} onClick={() => setPanel(panel === 'xray' ? null : 'xray')}>Visão</button>
          <button
            type="button"
            aria-pressed={panel === 'dict'}
            title="Consulta o dicionário em português, o Wikcionário e a Wikipédia da palavra selecionada"
            onMouseDown={(event) => {
              event.preventDefault()
              const info = reader.grabSelection()
              openDictionary(info?.text)
            }}
          >
            Dicionário
          </button>
          <button type="button" onClick={() => { const cfi = reader.currentCfi(); if (cfi) onBookmark({ cfi, title: reader.chapterTitle || 'Marcador' }) }}>Marcar</button>
        </div>
      </header>

      <div className="reader-stage">
        {loadError || reader.status === 'error' ? (
          <p className="reader-error">{loadError || reader.error}</p>
        ) : (
          <>
            <div ref={reader.hostRef} className="epub-host" data-testid="epub-host" />
            {reader.status === 'loading' && <p className="reader-wait">Abrindo o livro…</p>}
            <button type="button" className="edge left" aria-label="Página anterior" onClick={() => void reader.prev()} />
            <button type="button" className="edge right" aria-label="Próxima página" onClick={() => { setTurns((value) => value + 1); void reader.next() }} />
          </>
        )}
        {panel && <button type="button" className="reader-scrim" aria-label="Fechar painel" onClick={() => setPanel(null)} />}
        {panel && (
          <aside className="margin-panel" data-testid="reader-drawer">
            {panel === 'toc' && (
              <>
                <h2>Sumário</h2>
                <label className="search-line">
                  Busca
                  <input value={query} aria-label="Buscar no livro" onChange={(event) => void runSearch(event.target.value)} />
                </label>
                {searching && <p>Buscando…</p>}
                {hits.length > 0 && (
                  <ul className="plain">
                    {hits.map((hit) => (
                      <li key={hit.cfi}><button type="button" onClick={() => void reader.goTo(hit.cfi)}>{hit.excerpt}</button></li>
                    ))}
                  </ul>
                )}
                <ul className="plain">
                  {reader.toc.map((item) => (
                    <li key={`${item.href}-${item.label}`}><button type="button" onClick={() => void reader.goTo(item.href)}>{item.label}</button></li>
                  ))}
                </ul>
                {bookmarks.length > 0 && <h2>Marcadores</h2>}
                <ul className="plain">
                  {bookmarks.map((item) => (
                    <li key={item.id}>
                      <button type="button" onClick={() => void reader.goTo(item.cfi)}>{item.title}</button>
                      <button type="button" onClick={() => onRemoveBookmark(item.id)} aria-label="Remover marcador">×</button>
                    </li>
                  ))}
                </ul>
              </>
            )}
            {panel === 'look' && (
              <>
                <h2>Aparência</h2>
                <div className="theme-row">
                  {Object.entries(THEMES).map(([id, item]) => (
                    <button key={id} type="button" data-testid={`theme-${id}`} aria-pressed={prefs.theme === id} onClick={() => onPrefs({ theme: id })} style={{ background: item.bg, color: item.fg }}>
                      {item.label}
                    </button>
                  ))}
                </div>
                <div className="text-toggle">
                  {Object.entries(FONTS).map(([id, item]) => (
                    <button key={id} type="button" aria-pressed={prefs.fontFamily === id} onClick={() => onPrefs({ fontFamily: id })}>{item.label}</button>
                  ))}
                </div>
                <label>Peso {prefs.fontWeight}<input type="range" min={380} max={650} step={10} value={prefs.fontWeight} aria-label="Peso da fonte" onChange={(event) => onPrefs({ fontWeight: Number(event.target.value) })} /></label>
                <label>Tamanho {prefs.fontSize}<input type="range" min={14} max={32} value={prefs.fontSize} aria-label="Tamanho da fonte" onChange={(event) => onPrefs({ fontSize: Number(event.target.value) })} /></label>
                <label>Entrelinha {prefs.lineHeight.toFixed(2)}<input type="range" min={1.2} max={2.1} step={0.05} value={prefs.lineHeight} aria-label="Entrelinha" onChange={(event) => onPrefs({ lineHeight: Number(event.target.value) })} /></label>
                <label>Medida {prefs.measure}<input type="range" min={28} max={120} step={4} value={prefs.measure} aria-label="Largura da página" onChange={(event) => onPrefs({ measure: Number(event.target.value) })} /></label>
                <div className="text-toggle">
                  <button type="button" aria-pressed={prefs.align !== 'left'} onClick={() => onPrefs({ align: 'justify' })}>Justificado</button>
                  <button type="button" aria-pressed={prefs.align === 'left'} onClick={() => onPrefs({ align: 'left' })}>À esquerda</button>
                  <button type="button" aria-pressed={prefs.flow === 'paginated'} onClick={() => onPrefs({ flow: 'paginated' })}>Páginas</button>
                  <button type="button" aria-pressed={prefs.flow === 'scrolled'} onClick={() => onPrefs({ flow: 'scrolled' })}>Rolo</button>
                </div>
              </>
            )}
            {panel === 'xray' && (
              <>
                <h2>Visão do livro</h2>
                <p>Sessão {minutes}:{rest}{pace ? ` · ${pace} palavras/min` : ''}</p>
                <p>{overview ? `${overview.words} palavras no arquivo · cerca de ${overview.minutesLeft} min restantes` : 'Contando o texto…'}</p>
                <ul className="plain">
                  {(overview?.names ?? []).map((name) => (
                    <li key={name.name}><span>{name.name}</span> <em>{name.count}</em></li>
                  ))}
                </ul>
              </>
            )}
            {panel === 'dict' && (
              <div data-testid="dictionary-panel">
                <header className="drawer-head">
                  <h2>Dicionário</h2>
                  <button type="button" onClick={() => setPanel(null)}>Fechar</button>
                </header>
                <p className="drawer-lead">Consulta o dicionário em português, o Wikcionário e a Wikipédia da palavra selecionada.</p>
                <label className="search-line">
                  Palavra
                  <input
                    value={dictQuery}
                    placeholder="Buscar palavra no dicionário..."
                    aria-label="Buscar palavra no dicionário"
                    onChange={(event) => setDictQuery(event.target.value)}
                    onKeyDown={(event) => { if (event.key === 'Enter') void runLookup(dictQuery) }}
                  />
                </label>
                <button type="button" className="solid" onClick={() => void runLookup(dictQuery)}>{defining ? 'Consultando…' : 'Consultar'}</button>
                {!definition && !defining && <p className="drawer-lead">Escolha uma palavra no texto ou escreva acima.</p>}
                {definition && (
                  <article data-testid="definition">
                    <h3 className="headword">
                      {definition.word}
                      {definition.partOfSpeech ? <em> · {definition.partOfSpeech}</em> : null}
                    </h3>
                    <h4>Significado</h4>
                    {definition.senses.length === 0 ? (
                      <p>Sem significado no dicionário local nem no Wikcionário.</p>
                    ) : definition.senses.map((sense) => <p key={sense}>{sense}</p>)}
                    <h4>Wikipédia / Contexto</h4>
                    {definition.wikipediaExtract ? (
                      <>
                        {definition.wikipediaTitle ? <p className="dict-kicker">{definition.wikipediaTitle}</p> : null}
                        <p>{definition.wikipediaExtract}</p>
                        {definition.wikipediaUrl ? <a href={definition.wikipediaUrl} target="_blank" rel="noreferrer">Abrir na Wikipédia</a> : null}
                        {definition.source === 'local' ? <p className="drawer-lead">Contexto local. Na aplicação, a Wikipédia entra quando há rede.</p> : null}
                      </>
                    ) : <p>Nenhum verbete encontrado.</p>}
                    <button type="button" onClick={() => void highlightLookedUp()}>Grifar palavra no texto</button>
                    {dictMessage ? <p className="drawer-lead">{dictMessage}</p> : null}
                  </article>
                )}
              </div>
            )}
          </aside>
        )}
      </div>

      {reader.selection && (
        <div className="selection-bar" role="toolbar" aria-label="Ações da seleção" data-testid="selection-bar">
          <p className="selection-quote"><span>Seleção</span>“{reader.selection.text.slice(0, 96)}”</p>
          <div className="swatches">
            {Object.entries(HIGHLIGHTS).map(([id, ink]) => (
              <button key={id} type="button" aria-label={`Cor ${id}`} aria-pressed={color === id} style={{ background: ink }} onClick={() => setColor(id)} />
            ))}
          </div>
          <label>
            Nota à margem
            <input value={note} placeholder="opcional" aria-label="Nota à margem" onChange={(event) => setNote(event.target.value)} />
          </label>
          <button type="button" className="solid" onMouseDown={(event) => event.preventDefault()} onClick={() => { onHighlight({ cfi: reader.selection!.cfi, text: reader.selection!.text, note, color }); setNote(''); reader.clearSelection() }}>Grifar</button>
          <button type="button" onMouseDown={(event) => event.preventDefault()} onClick={() => openDictionary(reader.selection?.text)}>Dicionário</button>
          <button type="button" onClick={dismissSelection}>Fechar</button>
        </div>
      )}

      <footer className="reader-chrome foot">
        <button type="button" onClick={() => void reader.prev()}>Anterior</button>
        <span data-testid="progress-label">{Math.round(reader.percentage * 100)}%</span>
        <i className="always-meter" aria-hidden="true"><span style={{ width: `${Math.round(reader.percentage * 100)}%` }} /></i>
        <span>{reader.pageLabel}</span>
        <button type="button" onClick={() => { setTurns((value) => value + 1); void reader.next() }}>Seguinte</button>
      </footer>
      <i className="always-meter fixed" aria-hidden="true"><span style={{ width: `${Math.round(reader.percentage * 100)}%` }} /></i>
    </section>
  )
}
