import { expect, test, type Page } from '@playwright/test'

const NIGHT = 'rgb(28, 28, 26)'
const NIGHT_INK = 'rgb(242, 239, 233)'

test('estante, leitor, caderno e exemplo da nuvem', async ({ page }) => {
  await page.goto('/')
  await expect(page.getByTestId('bookshelf')).toBeVisible()
  await expect(page.getByRole('button', { name: 'Abrir O Farol da Ilha Rasa' })).toBeVisible()
  await expect(page.getByRole('button', { name: 'Abrir A Mesa da Cozinha' })).toBeVisible()

  await page.getByRole('button', { name: 'Abrir O Farol da Ilha Rasa' }).click()
  await expect(page.getByTestId('reader')).toBeVisible()
  const frame = page.frameLocator('iframe').first()
  await expect(frame.getByText('Na Ilha Rasa o vento', { exact: false })).toBeVisible()

  await page.getByRole('button', { name: 'Aparência da leitura' }).click()
  await page.getByRole('button', { name: 'Carvão' }).click()
  await expect(page.getByTestId('reader')).toHaveAttribute('data-theme', 'night')

  await page.getByRole('button', { name: 'Sumário' }).click()
  const drawer = page.getByTestId('reader-drawer')
  await expect(drawer.getByRole('heading', { name: 'Sumário' })).toBeVisible()
  const chapter = drawer.getByRole('button', { name: 'A primeira luz' })
  await expect(chapter).toBeVisible()
  await expect(drawer).toHaveCSS('background-color', NIGHT)
  await expect(chapter).toHaveCSS('color', NIGHT_INK)
  await expect(page.getByTestId('selection-bar')).toHaveCount(0)
  await page.screenshot({ path: 'test-results/sumario-carvao.png' })

  await page.getByRole('button', { name: 'Dicionário' }).click()
  await expect(page.getByTestId('dictionary-panel')).toBeVisible()
  await expect(page.getByText('Consulta o dicionário em português', { exact: false })).toBeVisible()
  await page.getByLabel('Buscar palavra no dicionário').fill('farol')
  await page.getByRole('button', { name: 'Consultar' }).click()
  const entry = page.getByTestId('definition')
  await expect(entry).toBeVisible()
  await expect(entry.getByRole('heading', { name: /farol/i })).toBeVisible()
  await expect(entry.getByRole('heading', { name: 'Significado' })).toBeVisible()
  await expect(entry.getByRole('heading', { name: 'Wikipédia / Contexto' })).toBeVisible()
  await expect(entry.getByText('Torre com um facho de luz', { exact: false })).toBeVisible()
  await expect(page.getByRole('button', { name: 'Grifar palavra no texto' })).toBeVisible()
  await expect(page.getByLabel('Nota à margem')).toHaveCount(0)
  const dictDrawer = page.getByTestId('reader-drawer')
  await expect(dictDrawer).toHaveCSS('background-color', NIGHT)
  await expect(entry.getByRole('heading', { name: /farol/i })).toHaveCSS('color', NIGHT_INK)
  const box = await dictDrawer.boundingBox()
  expect(box && box.x).toBeGreaterThan(700)
  await page.screenshot({ path: 'test-results/dicionario-carvao.png' })

  await page.getByRole('button', { name: 'Sumário' }).click()
  await page.getByRole('button', { name: 'O diário do faroleiro' }).click()
  await expect(frame.getByText('cadernos de três gerações', { exact: false })).toBeVisible()
  await expect(page.getByTestId('progress-label')).not.toHaveText('0%')

  const selected = await selectWord(page, 'farol')
  expect(selected).toBe(true)
  const bar = page.getByTestId('selection-bar')
  await expect(bar).toBeVisible()
  await expect(bar).toHaveCSS('background-color', NIGHT)
  await expect(bar.getByLabel('Nota à margem')).toHaveValue('')
  await expect(bar.getByRole('button', { name: 'Dicionário' })).toBeVisible()
  await page.screenshot({ path: 'test-results/selecao-carvao.png' })

  await page.getByRole('button', { name: 'Voltar à estante' }).click()
  await page.getByRole('button', { name: 'Caderno de Grifos' }).click()
  await expect(page.getByTestId('notebook')).toBeVisible()

  await page.getByRole('button', { name: 'Nuvem Amazon' }).click()
  await expect(page.getByRole('button', { name: 'Entrar com a Amazon' })).toBeVisible()
  await page.getByText('Opções avançadas').click()
  await page.getByRole('button', { name: 'Carregar exemplo' }).click()
  await expect(page.getByRole('cell', { name: 'Maré de Pedra' })).toBeVisible()
  await expect(page.getByRole('cell', { name: 'B0CLOUD001' })).toBeVisible()
  await expect(page.getByText('42%', { exact: true })).toBeVisible()
})

async function selectWord(page: Page, word: string) {
  const frames = page.frames().filter((frame) => frame !== page.mainFrame())
  for (const frame of frames) {
    const hit = await frame.evaluate((needle) => {
      if (!document.body) return false
      const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT)
      let node = walker.nextNode()
      while (node) {
        const text = node.textContent ?? ''
        const index = text.toLowerCase().indexOf(needle)
        if (index >= 0) {
          const range = document.createRange()
          range.setStart(node, index)
          range.setEnd(node, index + needle.length)
          const selection = window.getSelection()
          selection?.removeAllRanges()
          selection?.addRange(range)
          document.dispatchEvent(new Event('selectionchange'))
          return true
        }
        node = walker.nextNode()
      }
      return false
    }, word).catch(() => false)
    if (hit) return true
  }
  return false
}
