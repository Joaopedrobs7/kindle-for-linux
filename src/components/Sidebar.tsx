export type View = 'shelf' | 'notebook' | 'cloud' | 'settings'

const ITEMS: { id: View; label: string }[] = [
  { id: 'shelf', label: 'Biblioteca' },
  { id: 'cloud', label: 'Nuvem Amazon' },
  { id: 'notebook', label: 'Caderno de Grifos' },
  { id: 'settings', label: 'Configurações' },
]

export function Sidebar({ view, onNavigate }: { view: View; onNavigate: (view: View) => void }) {
  return (
    <aside className="sidebar">
      <p className="mark">
        <span>Kindle</span>
        para Linux
      </p>
      <nav>
        {ITEMS.map((item) => (
          <button key={item.id} type="button" aria-current={view === item.id ? 'page' : undefined} onClick={() => onNavigate(item.id)}>
            {item.label}
          </button>
        ))}
      </nav>
    </aside>
  )
}
