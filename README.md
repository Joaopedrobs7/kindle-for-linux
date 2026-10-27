# Kindle para Linux

## Instalação rápida (Ubuntu / Debian)

Um comando baixa o pacote publicado e instala o **Kindle para Linux**:

```bash
curl -fsSL https://raw.githubusercontent.com/Joaopedrobs7/kindle-for-linux/main/install.sh | sudo bash
```

O mesmo instalador está no GitHub Pages:

```bash
curl -fsSL https://joaopedrobs7.github.io/kindle-for-linux/install.sh | sudo bash
```

Quando o repositório APT responde, o script grava `/etc/apt/sources.list.d/kindle-linux.list`. A partir daí o apt instala e atualiza o pacote:

```bash
echo 'deb [trusted=yes] https://joaopedrobs7.github.io/kindle-for-linux ./' | sudo tee /etc/apt/sources.list.d/kindle-linux.list
sudo apt update
sudo apt install kindle-para-linux
```

O atalho **Kindle para Linux** fica no menu de aplicações. No terminal, o comando é `kindle-linux`. O pacote é amd64 e depende de `libwebkit2gtk-4.1-0` e `libgtk-3-0`.

Cliente de desktop em Rust e Tauri para ler, na mesma estante, a biblioteca Kindle na nuvem e EPUBs locais.

A conta Amazon abre numa janela WebKit com User-Agent de Chrome. Os cookies da sessão (`ubid-acbbr`, `x-acbbr`, `at-acbbr`, `session-id`, `session-token`, `x-main`, `at-main`, `uhura`, `deviceSessionToken`) ficam no SQLite deste computador. O aplicativo busca a biblioteca, a posição Whispersync e os destaques pelos endpoints do Kindle Cloud Reader. O texto protegido continua no leitor oficial, só que sem a barra da Amazon. Não há remoção de DRM.

## Requisitos

- Linux x64
- Rust estável e Node.js 22
- WebKitGTK 4.1 (`libwebkit2gtk-4.1-dev`)

## Desenvolvimento

```bash
npm install
npm run dev
```

A interface sobe em `http://127.0.0.1:43123`. Para a janela nativa, com a estante lendo `~/Documents/Kindle` e `/workspace/kindle`:

```bash
npm run dev:desktop
```

Na primeira abertura a pasta `/workspace/kindle` já pode trazer os EPUBs de amostra: **O Farol da Ilha Rasa**, **A Mesa da Cozinha** e **Cartas da Serra**.

## Entrar com a conta Amazon

1. Abra **Nuvem Amazon**.
2. Escolha **Brasil** (`ler.amazon.com.br`) ou **EUA** (`read.amazon.com`).
3. Clique em **Entrar no Kindle Cloud** e conclua senha, verificação em duas etapas ou passkey.
4. Clique em **Capturar sessão**.
5. Se preferir, cole o cabeçalho `Cookie` e use **Guardar cookies**.
6. **Sincronizar** chama `kindle-library/search` e pagina os resultados. **Anotações** chama `startReading` e `getAnnotations`.
7. **Ler** abre o livro em `/?asin=…` com uma folha de estilo que esconde o cromo da Amazon.

**Carregar exemplo** usa a fixture offline (`src-tauri/fixtures`) para ver a estante, o percentual e os grifos sem credenciais. Os testes fazem o mesmo caminho.

## Leitor

Páginas ou rolo, sumário, busca, quatro temas (Papel Quente, Sépia Editorial, Carvão, Tinta E-Ink), fonte serifada, grotesca ou mono, peso, tamanho, entrelinha, medida e alinhamento. Selecione uma palavra para grifar ou anotar. **Dicionário** abre um painel com o significado (léxico local e, com rede, Wikcionário) e o contexto da Wikipédia. **Visão** mostra nomes recorrentes, palavras e o tempo restante. Os painéis do leitor usam a cor do tema, inclusive no Carvão.

O **Caderno de grifos** exporta Markdown e `My Clippings.txt`.

## Atalhos

| Tecla | Ação |
| --- | --- |
| `F11` | Tela cheia |
| `Esc` | Fechar painéis |
| `←` `→` `Espaço` | Virar a página |
| `Ctrl+F` | Buscar |
| `Ctrl+B` | Marcar a página |
| `Ctrl++` `Ctrl+-` | Tamanho da letra |
| `Ctrl+1` / `Ctrl+2` | Estante / Nuvem |

## Testes

```bash
cargo test --manifest-path src-tauri/Cargo.toml
npm test
npm run test:e2e
```

`cargo test` confere a amostra EPUB, a junção paginada da biblioteca, a posição Whispersync, os destaques e a exportação do caderno, sem rede.

## Pacotes Linux

```bash
npm run dist:linux
```

O Tauri gera AppImage e deb. O pacote Debian pede WebKitGTK 4.1.

## Aviso

Kindle e Amazon são marcas de seus respectivos donos. Este é um cliente independente para leitura pessoal.
