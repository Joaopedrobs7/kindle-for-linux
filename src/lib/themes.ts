import type { ReaderPrefs } from '@/types'

export const THEMES: Record<string, { label: string; bg: string; fg: string; muted: string }> = {
  paper: { label: 'Papel Quente', bg: '#F7F5F0', fg: '#1C1A17', muted: '#6E6A62' },
  sepia: { label: 'Sépia Editorial', bg: '#F3E6D0', fg: '#2C2416', muted: '#7A6548' },
  night: { label: 'Carvão', bg: '#141413', fg: '#E7E1D6', muted: '#A39E93' },
  eink: { label: 'Tinta E-Ink', bg: '#EEF1EA', fg: '#1A1F18', muted: '#5C6758' },
}

export const FONTS: Record<string, { label: string; stack: string }> = {
  serif: { label: 'Serifada', stack: 'Literata, "Iowan Old Style", Palatino, serif' },
  sans: { label: 'Grotesca', stack: '"Source Sans 3", "Helvetica Neue", sans-serif' },
  mono: { label: 'Mono', stack: '"IBM Plex Mono", ui-monospace, monospace' },
}

export const HIGHLIGHTS: Record<string, string> = {
  amber: '#E4C56A',
  moss: '#B7C7A3',
  slate: '#A9B7C9',
  rose: '#E3B7B2',
}

export function themeOf(id: string) {
  return THEMES[id] ?? THEMES.paper
}

export function fontStack(id: string) {
  return (FONTS[id] ?? FONTS.serif).stack
}

export function themeRules(prefs: ReaderPrefs) {
  const theme = themeOf(prefs.theme)
  const align = prefs.align === 'left' ? 'left' : 'justify'
  return {
    body: {
      background: `${theme.bg} !important`,
      color: `${theme.fg} !important`,
      'font-family': `${fontStack(prefs.fontFamily)} !important`,
      'font-weight': `${prefs.fontWeight} !important`,
      'line-height': `${prefs.lineHeight} !important`,
      'text-align': `${align} !important`,
      padding: `0 ${prefs.measure}px !important`,
    },
    p: { 'text-align': `${align} !important` },
    h1: { 'font-weight': '500 !important', 'text-align': 'left !important' },
  }
}
