import type { Book } from '@/types'

const PAPERS = ['#E7E1D4', '#E4DDD2', '#D9E0D8', '#E8DCD2', '#DDD6C8']

function tone(title: string) {
  let index = 0
  for (const char of title) index = (index + char.charCodeAt(0)) % PAPERS.length
  return PAPERS[index]
}

export function Cover({ book }: { book: Book }) {
  const percent = Math.round(Math.min(1, Math.max(0, book.percentage)) * 100)
  return (
    <div className="cover">
      {book.cover ? (
        <img src={book.cover} alt="" referrerPolicy="no-referrer" loading="lazy" />
      ) : (
        <div className="cover-type" style={{ background: tone(book.title) }}>
          <strong>{book.title}</strong>
          <em>{book.author}</em>
        </div>
      )}
      <i className="spine" aria-hidden="true" />
      <div className="meter" aria-hidden="true">
        <span style={{ width: `${percent}%` }} />
      </div>
    </div>
  )
}
