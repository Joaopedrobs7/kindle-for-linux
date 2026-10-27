# Kindle para Linux

Kindle Cloud e EPUB na mesma estante.

![Biblioteca com livros da nuvem e EPUBs locais](docs/estante_unificada.webp)

![Leitor no Carvão, dicionário aberto em “farol”](docs/dicionario_carvao.webp)

![Sumário no tema Carvão](docs/sumario_carvao.webp)

![Caderno de grifos](docs/caderno_de_grifos.webp)

## Instalar

Linux / macOS:

```bash
curl -fsSL https://joaopedrobs7.github.io/kindle-for-linux/install.sh | sh
```

Windows (PowerShell):

```bash
irm https://joaopedrobs7.github.io/kindle-for-linux/install.ps1 | iex
```

Os arquivos (`.deb`, `.AppImage`, `.dmg`, `.msi`) estão nas [Releases](https://github.com/Joaopedrobs7/kindle-for-linux/releases).

- Kindle Cloud (Brasil ou EUA) e EPUB local na mesma estante
- Temas Papel, Sépia, Carvão e E-Ink
- Dicionário offline e Wikipédia
- Grifos em `My Clippings.txt` e Markdown

## Rodar do código

```bash
npm install && npm run dev:desktop
npm run dist:linux
```
