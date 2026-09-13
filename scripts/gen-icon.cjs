// Generate a simple 512x512 app icon (dark blue-purple bg + coral "D" note shape)
// Pure Node, no deps — creates source PNG for `tauri icon`
const zlib = require("zlib");
const fs = require("fs");
const path = require("path");

const SIZE = 512;
const outDir = path.join(__dirname, "..", "src-tauri", "icons-src");

// RGBA buffer
const buf = Buffer.alloc(SIZE * SIZE * 4);

function px(x, y, r, g, b, a = 255) {
  const i = (y * SIZE + x) * 4;
  buf[i] = r;
  buf[i + 1] = g;
  buf[i + 2] = b;
  buf[i + 3] = a;
}

// Background gradient: #1a1a2e -> #0f3460
for (let y = 0; y < SIZE; y++) {
  const t = y / SIZE;
  const r = Math.round(0x1a + (0x0f - 0x1a) * t);
  const g = Math.round(0x1a + (0x34 - 0x1a) * t);
  const b = Math.round(0x2e + (0x60 - 0x2e) * t);
  for (let x = 0; x < SIZE; x++) {
    px(x, y, r, g, b);
  }
}

// Rounded corner alpha (radius ~96)
function inRound(x, y, r) {
  const cx = Math.min(Math.max(x, r), SIZE - r);
  const cy = Math.min(Math.max(y, r), SIZE - r);
  const dx = x - cx, dy = y - cy;
  return dx * dx + dy * dy <= r * r;
}
const R = 96;
for (let y = 0; y < SIZE; y++) {
  for (let x = 0; x < SIZE; x++) {
    if (!inRound(x, y, R)) buf[(y * SIZE + x) * 4 + 3] = 0;
  }
}

// Draw a coral "#e94560" musical note (circle head + stem)
const HEAD_CX = 236, HEAD_CY = 268, HEAD_R = 52;
const STEM_X0 = 236 + 52, STEM_X1 = 236 + 60;
const STEM_Y0 = 268 - 20, STEM_Y1 = 268 - 130;
const FLAG = 40;
const C = [0xe9, 0x45, 0x60];
for (let y = 0; y < SIZE; y++) {
  for (let x = 0; x < SIZE; x++) {
    // head circle
    const hdx = x - HEAD_CX, hdy = y - HEAD_CY;
    if (hdx * hdx + hdy * hdy <= HEAD_R * HEAD_R) {
      px(x, y, ...C);
      continue;
    }
    // stem
    if (x >= STEM_X0 && x <= STEM_X1 && y >= STEM_Y0 && y <= STEM_Y1) {
      px(x, y, ...C);
      continue;
    }
    // flag: triangular curve top-right of stem
    const fy = STEM_Y0;
    const fx = STEM_X1;
    const dx = x - fx, dy = y - fy;
    if (dx >= 0 && dx <= FLAG * 2 && dy >= -FLAG && dy <= 0) {
      const ex = FLAG + (dy + FLAG); // expands with height
      if (dx <= ex) px(x, y, ...C);
    }
  }
}

function crc32(b) {
  let c;
  const table = [];
  for (let n = 0; n < 256; n++) {
    c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c >>> 0;
  }
  let crc = 0xffffffff;
  for (let i = 0; i < b.length; i++) crc = table[(crc ^ b[i]) & 0xff] ^ (crc >>> 8);
  return (crc ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const t = Buffer.from(type, "ascii");
  const crcBuf = Buffer.alloc(4);
  crcBuf.writeUInt32BE(crc32(Buffer.concat([t, data])));
  return Buffer.concat([len, t, data, crcBuf]);
}

const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(SIZE, 0);
ihdr.writeUInt32BE(SIZE, 4);
ihdr[8] = 8;  // bit depth
ihdr[9] = 6;  // color type RGBA
const raw = Buffer.alloc(SIZE * (SIZE * 4 + 1));
for (let y = 0; y < SIZE; y++) {
  raw[y * (SIZE * 4 + 1)] = 0; // filter: none
  buf.copy(raw, y * (SIZE * 4 + 1) + 1, y * SIZE * 4, (y + 1) * SIZE * 4);
}
const idat = zlib.deflateSync(raw);

const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk("IHDR", ihdr),
  chunk("IDAT", idat),
  chunk("IEND", Buffer.alloc(0)),
]);

fs.mkdirSync(outDir, { recursive: true });
fs.writeFileSync(path.join(outDir, "icon.png"), png);
console.log("Icon written:", path.join(outDir, "icon.png"));
