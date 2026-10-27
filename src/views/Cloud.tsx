import { useState } from 'react'
import type { Book, SessionStatus } from '@/types'

export function Cloud({
  books,
  session,
  busy,
  message,
  onRegion,
  onLogin,
  onCapture,
  onImport,
  onSync,
  onExample,
  onOpen,
  onAnnotations,
}: {
  books: Book[]
  session: SessionStatus
  busy: boolean
  message: string
  onRegion: (region: string) => void
  onLogin: () => void
  onCapture: () => void
  onImport: (header: string) => void
  onSync: () => void
  onExample: () => void
  onOpen: (asin: string) => void
  onAnnotations: (asin: string) => void
}) {
  const [header, setHeader] = useState('')
  const cloudBooks = books.filter((book) => book.source === 'cloud')

  return (
    <section className="page" data-testid="cloud">
      <header className="page-head">
        <div>
          <h1>Nuvem Amazon</h1>
          <p>A sessão fica neste computador. O livro protegido abre no leitor oficial, com a barra da Amazon recolhida.</p>
        </div>
      </header>
      <div className="cloud-grid">
        <section>
          <h2>Sessão</h2>
          <div className="text-toggle" role="group" aria-label="Região da Amazon">
            <button type="button" aria-pressed={session.region !== 'us'} onClick={() => onRegion('br')}>Brasil</button>
            <button type="button" aria-pressed={session.region === 'us'} onClick={() => onRegion('us')}>EUA</button>
          </div>
          <p className="session-line">{session.signedIn ? 'Sessão guardada' : 'Sem sessão'}{session.cookieNames.length > 0 ? ` · ${session.cookieNames.join(', ')}` : ''}</p>
          <div className="button-row">
            <button type="button" className="solid" onClick={onLogin}>Entrar no Kindle Cloud</button>
            <button type="button" className="line" onClick={onCapture}>Capturar sessão</button>
          </div>
          <label className="cookie-field">
            Cabeçalho Cookie
            <textarea value={header} onChange={(event) => setHeader(event.target.value)} placeholder="session-id=…; ubid-acbbr=…; at-acbbr=…; uhura=…; deviceSessionToken=…" aria-label="Cabeçalho Cookie" />
          </label>
          <button type="button" className="line" onClick={() => onImport(header)}>Guardar cookies</button>
        </section>
        <section>
          <h2>Biblioteca</h2>
          <div className="button-row">
            <button type="button" className="solid" disabled={busy} onClick={onSync}>{busy ? 'Sincronizando…' : 'Sincronizar'}</button>
            <button type="button" className="line" onClick={onExample}>Carregar exemplo</button>
          </div>
          {message && <p className="notice" role="status">{message}</p>}
          {cloudBooks.length === 0 ? (
            <p className="empty">Nenhum título da nuvem ainda.</p>
          ) : (
            <table className="catalog">
              <thead>
                <tr><th>Título</th><th>Autor</th><th>ASIN</th><th>Lido</th><th></th></tr>
              </thead>
              <tbody>
                {cloudBooks.map((book) => (
                  <tr key={book.id}>
                    <td>{book.title}</td>
                    <td>{book.author}</td>
                    <td className="asin">{book.asin}</td>
                    <td>{Math.round(book.percentage * 100)}%</td>
                    <td className="row-actions">
                      <button type="button" onClick={() => onOpen(book.asin)}>Ler</button>
                      <button type="button" onClick={() => onAnnotations(book.asin)}>Anotações</button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </section>
      </div>
    </section>
  )
}
