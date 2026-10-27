import type { Book, SessionStatus } from '@/types'

export function Cloud({
  books,
  session,
  region,
  busy,
  message,
  onRegion,
  onLogin,
  onSync,
  onExample,
  onOpen,
  onAnnotations,
}: {
  books: Book[]
  session: SessionStatus
  region: string
  busy: boolean
  message: string
  onRegion: (region: string) => void
  onLogin: () => void
  onSync: () => void
  onExample: () => void
  onOpen: (asin: string) => void
  onAnnotations: (asin: string) => void
}) {
  const cloudBooks = books.filter((book) => book.source === 'cloud')

  return (
    <section className="page" data-testid="cloud">
      <header className="page-head">
        <div>
          <h1>Nuvem Amazon</h1>
          <p>Entre com a sua conta. A biblioteca sincroniza e os livros abrem na mesma janela.</p>
        </div>
      </header>
      <div className="text-toggle" role="group" aria-label="Região da Amazon">
        <button type="button" aria-pressed={region !== 'us'} onClick={() => onRegion('br')}>Brasil</button>
        <button type="button" aria-pressed={region === 'us'} onClick={() => onRegion('us')}>EUA</button>
      </div>
      <div className="button-row cloud-primary">
        {session.signedIn ? (
          <button type="button" className="solid" disabled={busy} onClick={onSync}>
            {busy ? 'Sincronizando…' : 'Conectado à Amazon · Sincronizar agora'}
          </button>
        ) : (
          <button type="button" className="solid" onClick={onLogin}>Entrar com a Amazon</button>
        )}
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
      <details className="advanced">
        <summary>Opções avançadas</summary>
        <div className="button-row">
          {session.signedIn && <button type="button" className="line" onClick={onLogin}>Entrar de novo</button>}
          <button type="button" className="line" onClick={onExample}>Carregar exemplo</button>
        </div>
      </details>
    </section>
  )
}
