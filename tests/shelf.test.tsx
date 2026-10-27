import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { Shelf } from '@/views/Shelf'
import { Notebook } from '@/views/Notebook'
import type { Book, Highlight } from '@/types'

const books: Book[] = [
  {
    id: 'local:/workspace/kindle/o-farol-da-ilha-rasa.epub',
    source: 'local',
    asin: '',
    title: 'O Farol da Ilha Rasa',
    author: 'Edição de amostra',
    filePath: '/workspace/kindle/o-farol-da-ilha-rasa.epub',
    cover: '',
    percentage: 0.4,
    chapterTitle: 'O diário do faroleiro',
    missing: false,
    addedAt: 1,
  },
  {
    id: 'cloud:B0CLOUD001',
    source: 'cloud',
    asin: 'B0CLOUD001',
    title: 'Maré de Pedra',
    author: 'Helena Vilar',
    filePath: '',
    cover: '',
    percentage: 0.42,
    chapterTitle: '',
    missing: false,
    addedAt: 2,
  },
]

const notes: Highlight[] = [
  {
    id: '1',
    bookId: 'cloud:B0CLOUD001',
    bookTitle: 'Maré de Pedra',
    author: 'Helena Vilar',
    source: 'cloud',
    cfi: '',
    text: 'O faroleiro não salva o mar.',
    note: 'frase do diário',
    color: 'amber',
    startPos: 120,
    endPos: 148,
    createdAt: 1,
  },
]

describe('estante', () => {
  it('mostra EPUB local e livro da nuvem com ASIN', () => {
    render(<Shelf books={books} query="" onQuery={() => undefined} onOpen={() => undefined} onRemove={() => undefined} />)
    expect(screen.getByRole('heading', { name: 'Biblioteca' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Abrir O Farol da Ilha Rasa' })).toBeInTheDocument()
    expect(screen.getByText('B0CLOUD001')).toBeInTheDocument()
    expect(screen.getByText('42%')).toBeInTheDocument()
  })
})

describe('caderno', () => {
  it('lista o grifo e a nota', () => {
    render(<Notebook highlights={notes} onNote={() => undefined} onRemove={() => undefined} onExport={() => undefined} />)
    expect(screen.getByText('O faroleiro não salva o mar.')).toBeInTheDocument()
    expect(screen.getByDisplayValue('frase do diário')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'My Clippings.txt' })).toBeInTheDocument()
  })
})
