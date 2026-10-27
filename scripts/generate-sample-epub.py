#!/usr/bin/env python3
"""Build the original sample EPUB and a geometric cover."""

import shutil
import struct
import zlib
from pathlib import Path
import zipfile

ROOT = Path(__file__).resolve().parents[1]
RESOURCES = ROOT / "resources"
PUBLIC = ROOT / "public" / "kindle"
KINDLE = ROOT / "kindle"

CHAPTERS = [
    (
        "capitulo1.xhtml",
        "A primeira luz",
        [
            "Na Ilha Rasa o vento não pede licença. Ele entra pela fresta da porta, mexe o pavio da lamparina e deixa na mesa um cheiro de sal que nenhuma faxina consegue tirar. Mara acendeu o farol antes do sol, como o pai fazia, contando os degraus da escada em voz baixa para não acordar o medo.",
            "Eram cento e doze degraus até a lanterna. No patamar de cima, o mar ainda era uma chapa escura, e a única coisa viva era o feixe que ela mesma acabava de soltar. A luz girava devagar, varrendo a barra, o recife e o pedaço de céu onde as gaivotas iam aparecer.",
            "O diário do faroleiro estava aberto na última página escrita. A letra do pai, miúda e teimosa, registrava a maré, o vento e um recado que não era para o serviço: “Se a luz falhar, não discuta com o mar. Desça, acenda a reserva e escreva a hora.” Mara passou o dedo na frase. A reserva era um lampião de querosene guardado no armário de ferro, ao lado de um mapa em que a ilha parecia menor do que na vida.",
            "De manhã cedo chegou o barco do correio, um ponto branco que o farol já conhecia. O marinheiro deixou pão, duas cartas e uma notícia: a tempestade que vinha do sul tinha mudado de ideia, mas o canal continuava traiçoeiro para quem confiasse só no olho. “Hoje a luz precisa estar generosa”, ele disse, e foi embora antes do café.",
            "Mara desceu, comeu o pão ainda quente e subiu de novo. No meio da escada encontrou uma pena de gaivota e guardou-a no bolso, sem saber por quê. Lá em cima ajustou o pavio, limpou o vidro com um pano que já tinha visto muitos invernos e ficou escutando o mecanismo. O farol da Ilha Rasa não era um monumento. Era um relógio que o mar aceitava, desde que alguém não faltasse ao turno.",
            "Quando o sol finalmente subiu, a luz do farol ficou quase invisível e mesmo assim continuou girando. Era esse o ofício: persistir sem plateia. Mara anotou no diário a hora, a maré e uma linha só sua: “A primeira luz saiu no horário. A ilha segue no lugar.”",
        ],
    ),
    (
        "capitulo2.xhtml",
        "O diário do faroleiro",
        [
            "O diário ocupava uma caixa de madeira com dobradiças de latão. Havia cadernos de três gerações, todos com a mesma mania de escrever o tempo como se o tempo pudesse ser convencido. Mara leu na rede, enquanto a chuva fina experimentava o telhado.",
            "O avô descrevia naufrágios com economia, como quem não quer gastar tinta nem tristeza. A avó, nos anos em que assumiu o turno, escrevia o nome dos barcos e o que eles levavam: sal, tecido, uma noiva, um piano desafinado. O pai escrevia pouco, mas deixava recados para o futuro, que agora era ela.",
            "Numa página solta, Mara achou um desenho da lanterna e, ao lado, a frase: “O faroleiro não salva o mar. Só impede que a noite seja inteira.” Ela copiou a frase num papel novo e pregou do lado da janela. O vento logo dobrou a ponta do papel, como se quisesse ler também.",
            "À tarde o rádio chiou. Uma voz de terra firme pediu confirmação de que o farol estava operando. Mara respondeu com o código antigo, devagar, e imaginou do outro lado uma mesa cheia de xícaras frias. “Ilha Rasa no ar”, disse. “Feixe normal. Maré enchendo.” A voz agradeceu e desapareceu no chiado.",
            "Antes de dormir ela subiu mais uma vez. A chuva tinha parado. No vidro da lanterna havia gotas grandes, e cada gota segurava um pedaço invertido da lua. Mara pensou que o diário não era um arquivo: era uma conversa entre pessoas que não se encontravam, mediada por uma escada e por um facho de luz.",
            "Escreveu: “Li os cadernos até a página em que o pai fala da noite inteira. Prometo que a noite de hoje será só pela metade.” Apagou a lamparina da cozinha e deixou o farol fazer o resto.",
        ],
    ),
    (
        "capitulo3.xhtml",
        "Maré de volta",
        [
            "A tempestade que tinha mudado de ideia voltou atrás da decisão. Veio sem teatro no começo, só um vento mais baixo, empurrando a água contra o cais pequeno onde ninguém mais atracava. Mara fechou as janelas, guardou o diário na caixa de latão e subiu com o lampião reserva, para o caso de a luz principal cansar.",
            "No meio do turno o mecanismo tossiu. Foi um som seco, de dente de engrenagem que não encontra o outro. O feixe hesitou, apagou por dois suspiros e voltou menor. Mara não discutiu com o mar. Desceu três degraus, acendeu a reserva e escreveu a hora, como o pai tinha mandado, equilibrando o caderno no joelho.",
            "A luz reserva era mais amarela e mais curta. Ainda assim atravessou a chuva até o recife, e isso bastava. Um barco que ela não esperava respondeu com dois lampejos. Mara devolveu dois, sem saber o código, mas certa de que alguma coisa humana tinha visto a outra.",
            "De madrugada o vento cedeu. O farol principal voltou a girar como se nada tivesse acontecido, e a reserva ficou fumegando, orgulhosa, até ela a apagar. No cais, o mar devolveu uma tábua, uma corda e a pena de gaivota que Mara tinha perdido do bolso sem perceber. A ilha, fiel ao nome, continuava rasa e teimosa no meio da água.",
            "Mara escreveu a última linha do turno com a letra misturada à do pai: “A maré voltou. O farol também. Quem estiver no canal, siga a luz amarela até ela ficar branca de novo.” Fechou o diário, desceu a escada contando os degraus ao contrário e pôs a chaleira no fogo.",
            "Lá fora, o dia vinha claro, e o Farol da Ilha Rasa seguia no horário, como uma frase que a família inteira tinha decidido não abandonar.",
        ],
    ),
]


def write_png(path: Path, width: int, height: int, paint) -> None:
    raw = bytearray()
    for y in range(height):
        raw.append(0)
        for x in range(width):
            raw.extend(paint(x, y))
    def chunk(tag: bytes, data: bytes) -> bytes:
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
    ihdr = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(bytes(raw), 9)) + chunk(b"IEND", b"")
    path.write_bytes(png)


def cover_pixel(x: int, y: int):
    w, h = 600, 900
    if y > 760:
        band = 18 + (y % 24)
        return (14, 48, 46, 255) if (y // 12) % 2 == 0 else (band, 70, 64, 255)
    if 250 < x < 350 and 280 < y < 700:
        return (243, 236, 223, 255)
    if 285 < x < 315 and 180 < y < 300:
        return (242, 193, 78, 255)
    if (x - 300) ** 2 + (y - 190) ** 2 < 46 ** 2:
        return (242, 193, 78, 255)
    if 292 < x < 308 and 620 < y < 700:
        return (198, 83, 20, 255)
    return (18, 52, 47, 255)


def chapter_html(title: str, paragraphs: list[str]) -> str:
    body = "\n".join(f"    <p>{paragraph}</p>" for paragraph in paragraphs)
    return f"""<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" lang="pt-BR" xml:lang="pt-BR">
  <head>
    <title>{title}</title>
    <link rel="stylesheet" type="text/css" href="styles.css"/>
  </head>
  <body>
    <h1>{title}</h1>
{body}
  </body>
</html>
"""


def build() -> None:
    RESOURCES.mkdir(parents=True, exist_ok=True)
    PUBLIC.mkdir(parents=True, exist_ok=True)
    cover = RESOURCES / "cover.png"
    write_png(cover, 600, 900, cover_pixel)
    epub_path = RESOURCES / "amostra.epub"
    items = {
        "META-INF/container.xml": """<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>
""",
        "OEBPS/styles.css": "body{margin:0;padding:0 1rem;}h1{font-weight:normal;}p{text-align:justify;}",
        "OEBPS/cover.png": cover.read_bytes(),
    }
    nav_items = "\n".join(
        f'        <li><a href="{filename}">{title}</a></li>' for filename, title, _ in CHAPTERS
    )
    items["OEBPS/nav.xhtml"] = f"""<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops" lang="pt-BR" xml:lang="pt-BR">
  <head><title>Sumário</title></head>
  <body>
    <nav epub:type="toc" id="toc">
      <h1>Sumário</h1>
      <ol>
{nav_items}
      </ol>
    </nav>
  </body>
</html>
"""
    manifest = [
        '<item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>',
        '<item id="style" href="styles.css" media-type="text/css"/>',
        '<item id="cover" href="cover.png" media-type="image/png" properties="cover-image"/>',
    ]
    spine = []
    for index, (filename, title, paragraphs) in enumerate(CHAPTERS, start=1):
        items[f"OEBPS/{filename}"] = chapter_html(title, paragraphs)
        manifest.append(f'<item id="c{index}" href="{filename}" media-type="application/xhtml+xml"/>')
        spine.append(f'<itemref idref="c{index}"/>')
    items["OEBPS/content.opf"] = f"""<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" unique-identifier="uid" version="3.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="uid">urn:uuid:kindle-linux-amostra-farol</dc:identifier>
    <dc:title>O Farol da Ilha Rasa</dc:title>
    <dc:creator>Edição de amostra</dc:creator>
    <dc:language>pt-BR</dc:language>
    <meta property="dcterms:modified">2026-01-01T00:00:00Z</meta>
    <meta name="cover" content="cover"/>
  </metadata>
  <manifest>
    {''.join(manifest)}
  </manifest>
  <spine>
    {''.join(spine)}
  </spine>
</package>
"""
    if epub_path.exists():
        epub_path.unlink()
    with zipfile.ZipFile(epub_path, "w") as archive:
        info = zipfile.ZipInfo("mimetype")
        info.compress_type = zipfile.ZIP_STORED
        info.date_time = (2026, 1, 1, 0, 0, 0)
        archive.writestr(info, "application/epub+zip")
        for name, content in items.items():
            payload = content if isinstance(content, bytes) else content.encode("utf-8")
            archive.writestr(name, payload, compress_type=zipfile.ZIP_DEFLATED)
    PUBLIC.mkdir(parents=True, exist_ok=True)
    KINDLE.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(epub_path, PUBLIC / "o-farol-da-ilha-rasa.epub")
    shutil.copyfile(epub_path, KINDLE / "o-farol-da-ilha-rasa.epub")
    print(epub_path)


if __name__ == "__main__":
    build()
