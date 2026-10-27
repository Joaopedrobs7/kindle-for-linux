import { useMemo, useState } from 'react'
import { Cover } from '@/components/Cover'
import type { Book } from '@/types'

export function Shelf({
  books,
  query,
  onQuery,
  onOpen,
  onRemove,
}: {
  books: Book[]
  query: string
  onQuery: (query: string) => void
  onOpen: (book: Book) => void
  onRemove: (book: Book) => void
}) {
  const [layout, setLayout] = useState<'grid' | 'list'>('grid')
  const visible = useMemo(() => {
    const needle = query.trim().toLowerCase()
    return books
      .filter((book) => !needle || `${book.title} ${book.author} ${book.asin}`.toLowerCase().includes(needle))
      .slice()
      .sort((a, b) => Number(b.percentage > 0) - Number(a.percentage > 0) || a.title.localeCompare(b.title, 'pt'))
  }, [books, query])
  const local = books.filter((book) => book.source !== 'cloud').length
  const cloud = books.length - local

  return (
    <section className="page" data-testid="bookshelf">
      <header className="page-head">
        <div>
          <h1>Biblioteca</h1>
          <p>{books.length === 1 ? '1 livro' : `${books.length} livros`} · {local} no disco · {cloud} na nuvem</p>
        </div>
        <div className="head-tools">
          <input value={query} onChange={(event) => onQuery(event.target.value)} placeholder="Título, autor, ASIN" aria-label="Buscar na estante" />
          <div className="text-toggle" role="group" aria-label="Disposição">
            <button type="button" aria-pressed={layout === 'grid'} onClick={() => setLayout('grid')}>Grade</button>
            <button type="button" aria-pressed={layout === 'list'} onClick={() => setLayout('list')}>Lista</button>
          </div>
        </div>
      </header>
      {visible.length === 0 ? (
        <p className="empty">Nenhum livro com esse nome. A pasta observada é <code>/workspace/kindle</code>.</p>
      ) : layout === 'grid' ? (
        <div className="grid">
          {visible.map((book) => (
            <article key={book.id} className="card" data-testid="book-card">
              <button type="button" className="cover-hit" aria-label={`Abrir ${book.title}`} onClick={() => onOpen(book)}>
                <Cover book={book} />
              </button>
              <div className="card-meta">
                <h2>{book.title}</h2>
                <p>{book.author}</p>
                <p className="meta-line">
                  {book.source === 'cloud' ? <span className="asin">{book.asin}</span> : <span>EPUB</span>}
                  <span>{Math.round(book.percentage * 100)}%</span>
                </p>
              </div>
              <button type="button" className="quiet-link" onClick={() => onRemove(book)}>Retirar</button>
            </article>
          ))}
        </div>
      ) : (
        <ul className="list">
          {visible.map((book) => (
            <li key={book.id}>
              <button type="button" onClick={() => onOpen(book)} aria-label={`Abrir ${book.title}`}>
                <Cover book={book} />
                <span>
                  <strong>{book.title}</strong>
                  <em>{book.author}</em>
                </span>
                <span className="meta-line">{book.asin || 'EPUB'} · {Math.round(book.percentage * 100)}%</span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  )
}
