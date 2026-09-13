// LRC / VTT 字幕解析工具

export interface SubtitleCue {
  /** 开始时间（秒） */
  start: number;
  /** 结束时间（秒） */
  end: number;
  /** 字幕文本 */
  text: string;
}

function parseTimeStamp(s: string): number {
  const m = s.match(/(?:(\d+):)?(\d{1,2}):(\d{2})(?:[.:](\d{1,3}))?/);
  if (!m) return -1;
  const h = m[1] ? parseInt(m[1], 10) : 0;
  const mm = parseInt(m[2], 10);
  const ss = parseInt(m[3], 10);
  const frac = m[4] ? parseInt(m[4].padEnd(3, "0").slice(0, 3), 10) / 1000 : 0;
  return h * 3600 + mm * 60 + ss + frac;
}

/** 解析标准 LRC 与增强 LRC。 */
export function parseLrc(content: string): SubtitleCue[] {
  const lines = content.split(/\r?\n/);
  const cues: SubtitleCue[] = [];

  for (const raw of lines) {
    const line = raw.trim();
    if (!line) continue;

    // 提取所有 [mm:ss.xx] 时间标签
    const tagRe = /\[(\d{1,2}:\d{1,2}(?:[.:]\d{1,3})?)\]/g;
    const stamps: number[] = [];
    let m: RegExpExecArray | null;
    while ((m = tagRe.exec(line)) !== null) {
      const t = parseTimeStamp(m[1]);
      if (t >= 0) stamps.push(t);
    }
    if (!stamps.length) continue;

    // 去除时间标签，得到正文
    let text = line.replace(tagRe, "");
    // 元数据标签（[ti:] [ar:] [al:] [offset:] 等）忽略
    if (/^\[[a-zA-Z]+:/.test(text.trim())) continue;
    // 增强 LRC 的 <mm:ss.xx> 字级时间标签：删除
    text = text.replace(/<\d{1,2}:\d{1,2}(?:[.:]\d{1,3})?>/g, "").trim();
    if (!text) continue;

    for (const t of stamps) {
      cues.push({ start: t, end: 0, text });
    }
  }

  cues.sort((a, b) => a.start - b.start);
  for (let i = 0; i < cues.length; i++) {
    cues[i].end = i < cues.length - 1 ? cues[i + 1].start : cues[i].start + 5;
  }
  return cues;
}

function parseVttTime(s: string): number {
  const m = s.trim().match(/(?:(\d+):)?(\d{1,2}):(\d{2})(?:[.:](\d{1,3}))?/);
  if (!m) return -1;
  const h = m[1] ? parseInt(m[1], 10) : 0;
  const mm = parseInt(m[2], 10);
  const ss = parseInt(m[3], 10);
  const ms = m[4] ? parseInt(m[4].padEnd(3, "0").slice(0, 3), 10) / 1000 : 0;
  return h * 3600 + mm * 60 + ss + ms;
}

/** 解析 WebVTT。 */
export function parseVtt(content: string): SubtitleCue[] {
  const blocks = content.split(/\r?\n\r?\n/);
  const cues: SubtitleCue[] = [];

  for (const block of blocks) {
    const lines = block.split(/\r?\n/).map((l) => l.trim());
    if (!lines.length || !lines[0]) continue;
    const first = lines[0];
    if (
      first.startsWith("WEBVTT") ||
      first.startsWith("NOTE") ||
      first.startsWith("STYLE") ||
      first.startsWith("REGION")
    ) {
      continue;
    }

    const timeIdx = lines.findIndex((l) => l.includes("-->"));
    if (timeIdx < 0) continue;
    const tm = lines[timeIdx].match(/(\S+)\s*-->\s*(\S+)/);
    if (!tm) continue;
    const start = parseVttTime(tm[1]);
    const end = parseVttTime(tm[2]);
    if (start < 0 || end < 0 || end <= start) continue;

    const textLines = lines.slice(timeIdx + 1).filter((l) => l.length > 0);
    if (!textLines.length) continue;
    // 去掉内联标签（<c> <b> <i> <v> 等）
    const clean = textLines.join("\n").replace(/<[^>]+>/g, "").trim();
    if (clean) {
      cues.push({ start, end, text: clean });
    }
  }

  return cues.sort((a, b) => a.start - b.start);
}

function parseSrtTime(s: string): number {
  const m = s.trim().match(/(?:(\d+):)?(\d{1,2}):(\d{2})(?:[,\.](\d{1,3}))?/);
  if (!m) return -1;
  const h = m[1] ? parseInt(m[1], 10) : 0;
  const mm = parseInt(m[2], 10);
  const ss = parseInt(m[3], 10);
  const ms = m[4] ? parseInt(m[4].padEnd(3, "0").slice(0, 3), 10) / 1000 : 0;
  return h * 3600 + mm * 60 + ss + ms;
}

/** 解析 SRT (SubRip) 字幕。 */
export function parseSrt(content: string): SubtitleCue[] {
  const normalized = content.replace(/\r\n/g, "\n").replace(/\r/g, "\n");
  const blocks = normalized.split(/\n\s*\n/);
  const cues: SubtitleCue[] = [];

  for (const block of blocks) {
    const lines = block.split("\n").map((l) => l.trim()).filter(Boolean);
    if (!lines.length) continue;

    const timeIdx = lines.findIndex((l) => l.includes("-->"));
    if (timeIdx < 0) continue;
    const tm = lines[timeIdx].match(/(\S+)\s*-->\s*(\S+)/);
    if (!tm) continue;

    const start = parseSrtTime(tm[1]);
    const end = parseSrtTime(tm[2]);
    if (start < 0 || end < 0 || end <= start) continue;

    const textLines = lines.slice(timeIdx + 1);
    if (!textLines.length) continue;

    // 过滤 HTML/样式标签（<i>, <b>, <font...>, etc.）
    const clean = textLines.join("\n").replace(/<[^>]+>/g, "").trim();
    if (clean) {
      cues.push({ start, end, text: clean });
    }
  }

  return cues.sort((a, b) => a.start - b.start);
}

/** 根据内容自动检测格式（LRC / VTT / SRT）并解析。 */
export function detectAndParse(content: string): SubtitleCue[] {
  if (!content || !content.trim()) return [];
  const head = content.slice(0, 2048).trimStart();
  if (/^WEBVTT/i.test(head)) {
    return parseVtt(content);
  }
  if (/\[\d{1,2}:\d{1,2}(?:[.:]\d{1,3})?\]/.test(content)) {
    return parseLrc(content);
  }
  if (content.includes("-->")) {
    const srtCues = parseSrt(content);
    if (srtCues.length > 0) return srtCues;
    return parseVtt(content);
  }
  return [];
}

/** 二分查找当前播放时间对应的活跃字幕行。 */
export function getActiveCue(cues: SubtitleCue[], time: number): SubtitleCue | null {
  if (!cues.length) return null;
  let lo = 0;
  let hi = cues.length - 1;
  let ans = -1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    if (cues[mid].start <= time) {
      ans = mid;
      lo = mid + 1;
    } else {
      hi = mid - 1;
    }
  }
  if (ans < 0) return null;
  const cue = cues[ans];
  return time <= cue.end ? cue : null;
}
