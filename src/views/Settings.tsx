import type { Settings } from '@/types'

export function SettingsView({
  settings,
  folders,
  onChange,
  onImport,
}: {
  settings: Settings
  folders: string[]
  onChange: (next: Settings) => void
  onImport: (path: string) => void
}) {
  return (
    <section className="page settings">
      <header className="page-head">
        <div>
          <h1>Configurações</h1>
          <p>Aparência da estante, região da Amazon e atalhos.</p>
        </div>
      </header>
      <h2>Aparência</h2>
      <div className="text-toggle" role="group" aria-label="Aparência">
        <button type="button" aria-pressed={settings.appearance !== 'ink'} onClick={() => onChange({ ...settings, appearance: 'paper' })}>Papel</button>
        <button type="button" aria-pressed={settings.appearance === 'ink'} onClick={() => onChange({ ...settings, appearance: 'ink' })}>Carvão</button>
      </div>
      <h2>Pastas observadas</h2>
      <ul className="folder-list">
        {folders.map((folder) => <li key={folder}><code>{folder}</code></li>)}
      </ul>
      <form
        className="import-row"
        onSubmit={(event) => {
          event.preventDefault()
          const data = new FormData(event.currentTarget)
          const path = String(data.get('path') ?? '')
          if (path.trim()) onImport(path.trim())
          event.currentTarget.reset()
        }}
      >
        <input name="path" aria-label="Caminho de um EPUB" placeholder="Caminho de um .epub" />
        <button type="submit" className="line">Importar</button>
      </form>
      <h2>Atalhos</h2>
      <ul className="keys">
        <li><kbd>F11</kbd> Tela cheia</li>
        <li><kbd>←</kbd> <kbd>→</kbd> <kbd>Espaço</kbd> Virar a página</li>
        <li><kbd>Ctrl</kbd> <kbd>F</kbd> Buscar no livro</li>
        <li><kbd>Ctrl</kbd> <kbd>B</kbd> Marcar a página</li>
        <li><kbd>Ctrl</kbd> <kbd>+</kbd> <kbd>−</kbd> Tamanho da letra</li>
      </ul>
    </section>
  )
}
