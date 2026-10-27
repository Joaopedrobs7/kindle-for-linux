import { useState } from 'react'
import { HIGHLIGHTS } from '@/lib/themes'
import type { Highlight } from '@/types'

export function Notebook({
  highlights,
  onNote,
  onRemove,
  onExport,
}: {
  highlights: Highlight[]
  onNote: (id: string, note: string) => void
  onRemove: (id: string) => void
  onExport: (format: 'markdown' | 'clippings') => void
}) {
  const [notice, setNotice] = useState('')
  const groups = new Map<string, Highlight[]>()
  for (const item of highlights) {
    const key = item.bookTitle || 'Sem título'
    groups.set(key, [...(groups.get(key) ?? []), item])
  }

  return (
    <section className="page" data-testid="notebook">
      <header className="page-head">
        <div>
          <h1>Caderno de grifos</h1>
          <p>{highlights.length === 0 ? 'Os destaques da estante e da nuvem ficam aqui.' : `${highlights.length} trechos`}</p>
        </div>
        <div className="head-tools">
          <button type="button" className="solid" onClick={() => void onExport('markdown')}>Markdown</button>
          <button type="button" className="line" onClick={() => void onExport('clippings')}>My Clippings.txt</button>
        </div>
      </header>
      {notice && <p className="notice">{notice}</p>}
      {highlights.length === 0 ? (
        <p className="empty">Ainda não há grifos. Abra um EPUB e selecione uma frase, ou sincronize as anotações da nuvem.</p>
      ) : (
        [...groups.entries()].map(([title, items]) => (
          <section key={title} className="clip-group">
            <h2>{title}</h2>
            <p className="by">{items[0]?.author}</p>
            <ul>
              {items.map((item) => (
                <li key={item.id}>
                  <blockquote style={{ borderColor: HIGHLIGHTS[item.color] ?? HIGHLIGHTS.amber }}>{item.text}</blockquote>
                  <textarea
                    value={item.note}
                    aria-label={`Nota de ${item.text.slice(0, 24)}`}
                    placeholder="Nota à margem"
                    onChange={(event) => onNote(item.id, event.target.value)}
                  />
                  <button type="button" className="quiet-link" onClick={() => { onRemove(item.id); setNotice('') }}>Apagar</button>
                </li>
              ))}
            </ul>
          </section>
        ))
      )}
    </section>
  )
}
