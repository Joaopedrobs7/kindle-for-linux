import type { Book, Highlight, SessionStatus, Settings, Snapshot } from '@/types'
import { EMPTY_SESSION } from '@/types'

const reader = {
  theme: 'paper',
  fontFamily: 'serif',
  fontWeight: 440,
  fontSize: 19,
  lineHeight: 1.65,
  measure: 68,
  align: 'justify',
  flow: 'paginated',
  columns: 2,
}

const settings: Settings = { appearance: 'paper', region: 'br', reader: { ...reader } }

const locals: Book[] = [
  book('local:/workspace/kindle/o-farol-da-ilha-rasa.epub', 'O Farol da Ilha Rasa', 'Edição de amostra', '/workspace/kindle/o-farol-da-ilha-rasa.epub'),
  book('local:/workspace/kindle/a-mesa-da-cozinha.epub', 'A Mesa da Cozinha', 'Clara Mendes', '/workspace/kindle/a-mesa-da-cozinha.epub'),
  book('local:/workspace/kindle/cartas-da-serra.epub', 'Cartas da Serra', 'Nuno Pires', '/workspace/kindle/cartas-da-serra.epub'),
]

let books: Book[] = locals.map((item) => ({ ...item }))
let highlights: Highlight[] = []
let bookmarks: Snapshot['bookmarks'] = []
let session: SessionStatus = { ...EMPTY_SESSION, cookieNames: [] }
let current = settings

function book(id: string, title: string, author: string, filePath: string): Book {
  return {
    id,
    source: 'local',
    asin: '',
    title,
    author,
    filePath,
    cover: '',
    percentage: 0,
    chapterTitle: '',
    missing: false,
    addedAt: 1,
  }
}

function snapshot(): Snapshot {
  return {
    books: books.map((item) => ({ ...item })),
    highlights: highlights.map((item) => ({ ...item })),
    bookmarks: bookmarks.map((item) => ({ ...item })),
    settings: { ...current, reader: { ...current.reader } },
    session: { ...session, cookieNames: [...session.cookieNames] },
    folders: ['/home/ubuntu/Documents/Kindle', '/workspace/kindle'],
  }
}

export const mock = {
  snapshot,
  async readBook(id: string) {
    const item = books.find((book) => book.id === id)
    const name = item?.filePath.split('/').pop()
    if (!name) throw new Error('Livro não encontrado')
    const response = await fetch(`/kindle/${name}`)
    if (!response.ok) throw new Error('Não foi possível abrir o EPUB')
    return Array.from(new Uint8Array(await response.arrayBuffer()))
  },
  async saveProgress(input: { bookId: string; cfi: string; percentage: number; chapterTitle?: string }) {
    books = books.map((item) =>
      item.id === input.bookId
        ? { ...item, percentage: input.percentage, chapterTitle: input.chapterTitle ?? item.chapterTitle }
        : item,
    )
  },
  async addHighlight(input: { bookId: string; cfi: string; text: string; note?: string; color?: string }) {
    const host = books.find((item) => item.id === input.bookId)
    const created: Highlight = {
      id: `h-${highlights.length + 1}`,
      bookId: input.bookId,
      bookTitle: host?.title ?? '',
      author: host?.author ?? '',
      source: 'local',
      cfi: input.cfi,
      text: input.text,
      note: input.note ?? '',
      color: input.color ?? 'amber',
      startPos: 0,
      endPos: 0,
      createdAt: Date.now(),
    }
    highlights = [created, ...highlights]
    return created
  },
  async updateHighlight(id: string, note: string) {
    highlights = highlights.map((item) => (item.id === id ? { ...item, note } : item))
  },
  async removeHighlight(id: string) {
    highlights = highlights.filter((item) => item.id !== id)
  },
  async addBookmark(input: { bookId: string; cfi: string; title: string }) {
    bookmarks = [{ id: `b-${bookmarks.length + 1}`, bookId: input.bookId, cfi: input.cfi, title: input.title, createdAt: Date.now() }, ...bookmarks]
  },
  async removeBookmark(id: string) {
    bookmarks = bookmarks.filter((item) => item.id !== id)
  },
  async removeBook(id: string) {
    books = books.filter((item) => item.id !== id)
    return snapshot()
  },
  async saveSettings(next: Settings) {
    current = { ...next, reader: { ...next.reader } }
    return snapshot()
  },
  async exportClippings(format: string) {
    if (format === 'clippings') {
      return highlights.map((item) => `${item.bookTitle} (${item.author})\n- Seu destaque\n\n${item.text}\n==========\n`).join('')
    }
    return ['# Caderno de grifos', '', ...highlights.flatMap((item) => [`## ${item.bookTitle}`, '', `> ${item.text}`, '', item.note, ''])].join('\n')
  },
  async defineWord(word: string) {
    const clean = word.trim().split(/\s+/)[0] ?? word
    const key = clean.toLowerCase().normalize('NFD').replace(/\p{Diacritic}/gu, '')
    const lexicon: Record<string, { pos: string; sense: string; wiki: string }> = {
      farol: {
        pos: 'substantivo masculino',
        sense: 'Torre com um facho de luz que orienta quem vem pelo mar.',
        wiki: 'Estrutura costeira que projeta luz para orientar a navegação durante a noite.',
      },
      mare: {
        pos: 'substantivo feminino',
        sense: 'Movimento periódico do mar, subida e descida da água.',
        wiki: 'Subida e descida do nível do mar ao longo do dia.',
      },
      lamparina: {
        pos: 'substantivo feminino',
        sense: 'Lâmpada pequena de pavio, usada quando a luz grande falha.',
        wiki: 'Luz pequena de pavio, comum quando a iluminação principal falha.',
      },
      gaivota: {
        pos: 'substantivo feminino',
        sense: 'Ave costeira, clara, que acompanha barcos e faróis.',
        wiki: 'Ave marinha de plumagem clara, frequente no litoral.',
      },
      diario: {
        pos: 'substantivo masculino',
        sense: 'Caderno em que se anota o dia: hora, maré, o que não se quer esquecer.',
        wiki: 'Registo quotidiano de factos, horas e observações.',
      },
    }
    const entry = lexicon[key]
    return {
      word: clean,
      partOfSpeech: entry?.pos ?? '',
      senses: entry ? [entry.sense] : [],
      wikipediaTitle: entry ? clean : '',
      wikipediaExtract: entry?.wiki ?? '',
      wikipediaUrl: '',
      source: entry ? 'local' : 'none',
    }
  },
  async bookOverview(bookId: string) {
    const item = books.find((book) => book.id === bookId)
    return { words: 980, names: [{ name: 'Mara', count: 12 }, { name: 'Ilha', count: 8 }], percentage: item?.percentage ?? 0, minutesLeft: 18 }
  },
  async cloudLoadExample() {
    const samples: Book[] = [
      { id: 'cloud:B0CLOUD001', source: 'cloud', asin: 'B0CLOUD001', title: 'Maré de Pedra', author: 'Helena Vilar', filePath: '', cover: '', percentage: 0.42, chapterTitle: '', missing: false, addedAt: 2 },
      { id: 'cloud:B0CLOUD002', source: 'cloud', asin: 'B0CLOUD002', title: 'O Caderno Azul', author: 'Nuno Pires', filePath: '', cover: '', percentage: 0.07, chapterTitle: '', missing: false, addedAt: 3 },
      { id: 'cloud:B0CLOUD003', source: 'cloud', asin: 'B0CLOUD003', title: 'Casa de Chá', author: 'Lina Ortega', filePath: '', cover: '', percentage: 0, chapterTitle: '', missing: false, addedAt: 4 },
    ]
    books = [...books.filter((item) => item.source !== 'cloud'), ...samples]
    highlights = [
      {
        id: 'cloud:B0CLOUD001:120:148',
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
        createdAt: 1730000001000,
      },
      ...highlights.filter((item) => item.source !== 'cloud'),
    ]
    return snapshot()
  },
  async cloudImport(header: string, region: string) {
    const names = header.split(';').map((part) => part.split('=')[0]?.trim()).filter(Boolean) as string[]
    session = { signedIn: names.some((name) => name === 'session-id' || name.startsWith('at-')), region, cookieNames: names, hasUhura: names.includes('uhura'), hasDeviceToken: names.includes('deviceSessionToken') }
    return snapshot()
  },
}
