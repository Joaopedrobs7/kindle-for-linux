import fs from 'node:fs'
import path from 'node:path'
import zlib from 'node:zlib'

const root = path.resolve(import.meta.dirname, '..')
const target = path.join(root, 'resources', 'icon.png')
fs.mkdirSync(path.dirname(target), { recursive: true })

const size = 512

function png(width, height, rgba) {
  const raw = Buffer.alloc((width * 4 + 1) * height)
  for (let y = 0; y < height; y += 1) {
    const start = y * (width * 4 + 1)
    raw[start] = 0
    rgba.copy(raw, start + 1, y * width * 4, (y + 1) * width * 4)
  }
  const ihdr = Buffer.alloc(13)
  ihdr.writeUInt32BE(width, 0)
  ihdr.writeUInt32BE(height, 4)
  ihdr[8] = 8
  ihdr[9] = 6
  return Buffer.concat([
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
    chunk('IHDR', ihdr),
    chunk('IDAT', zlib.deflateSync(raw, { level: 9 })),
    chunk('IEND', Buffer.alloc(0)),
  ])
}

function chunk(type, data) {
  const length = Buffer.alloc(4)
  length.writeUInt32BE(data.length)
  const body = Buffer.concat([Buffer.from(type), data])
  const crc = Buffer.alloc(4)
  crc.writeUInt32BE(crc32(body))
  return Buffer.concat([length, body, crc])
}

function crc32(buffer) {
  let crc = ~0
  for (const byte of buffer) {
    crc ^= byte
    for (let i = 0; i < 8; i += 1) crc = (crc >>> 1) ^ (0xedb88320 & -(crc & 1))
  }
  return ~crc >>> 0
}

const pixels = Buffer.alloc(size * size * 4)
const set = (x, y, r, g, b, a = 255) => {
  if (x < 0 || y < 0 || x >= size || y >= size) return
  const index = (y * size + x) * 4
  pixels[index] = r
  pixels[index + 1] = g
  pixels[index + 2] = b
  pixels[index + 3] = a
}

for (let y = 0; y < size; y += 1) {
  for (let x = 0; x < size; x += 1) {
    const dx = Math.max(Math.abs(x - 256) - 190, 0)
    const dy = Math.max(Math.abs(y - 256) - 190, 0)
    const outside = dx * dx + dy * dy > 58 * 58
    if (outside) set(x, y, 0, 0, 0, 0)
    else set(x, y, 35, 30, 26, 255)
    const inLeft = x > 150 && x < 250 && y > 96 && y < 400
    const inRight = x > 262 && x < 362 && y > 96 && y < 400
    const inSpine = x >= 246 && x <= 266 && y > 96 && y < 400
    if (inLeft) set(x, y, 244, 236, 223)
    if (inRight) set(x, y, 228, 213, 188)
    if (inSpine) set(x, y, 198, 83, 20)
    if (x > 300 && x < 338 && y > 96 && y < 176 && x + (y - 96) * 0.2 < 360) set(x, y, 224, 122, 61)
    if (inRight && y > 180 && y < 186 && x > 286 && x < 340) set(x, y, 176, 154, 126)
    if (inRight && y > 204 && y < 210 && x > 286 && x < 330) set(x, y, 176, 154, 126)
    if (inRight && y > 228 && y < 234 && x > 286 && x < 320) set(x, y, 176, 154, 126)
  }
}

fs.writeFileSync(target, png(size, size, pixels))
console.log(target)
