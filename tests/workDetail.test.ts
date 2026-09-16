import { test } from "node:test";
import assert from "node:assert/strict";
import {
  resolveMode,
  resolveActions,
  actionLabel,
  actionVariant,
  resolveStatusBadge,
  tagsFromNames,
  type ActionKey,
  type ActionInput,
} from "../src/lib/workDetail.ts";

// 便捷构造：只写关心的字段，其余用默认值
function input(p: Partial<ActionInput>): ActionInput {
  return {
    source: "work",
    mode: "local",
    hasOnline: false,
    hasLocalWork: false,
    hasRj: true,
    ...p,
  };
}

// ============ 模式判定（spec §2.2） ============
test("mode: ASMR ONE 页恒为 asmr（即使本地无音轨）", () => {
  assert.equal(
    resolveMode({ source: "asmr", hasOnline: false, localTrackCount: 0 }),
    "asmr"
  );
});

test("mode: 作品库本地有音轨 → local", () => {
  assert.equal(
    resolveMode({ source: "work", hasOnline: true, localTrackCount: 2 }),
    "local"
  );
});

test("mode: 作品库本地无音轨 + asmr.one 存在 → asmr", () => {
  assert.equal(
    resolveMode({ source: "work", hasOnline: true, localTrackCount: 0 }),
    "asmr"
  );
});

test("mode: 作品库本地无音轨 + asmr.one 不存在 → dlsite", () => {
  assert.equal(
    resolveMode({ source: "work", hasOnline: false, localTrackCount: 0 }),
    "dlsite"
  );
});

test("mode: DLsite 页 + asmr.one 存在 → asmr（场景 3）", () => {
  assert.equal(
    resolveMode({ source: "dlsite", hasOnline: true, localTrackCount: 0 }),
    "asmr"
  );
});

test("mode: DLsite 页 + asmr.one 不存在 → dlsite（场景 2）", () => {
  assert.equal(
    resolveMode({ source: "dlsite", hasOnline: false, localTrackCount: 0 }),
    "dlsite"
  );
});

// ============ 按钮矩阵（spec §4） ============
// 场景 1/3：ASMR ONE 或 DLsite 页，asmr.one 存在，未入库
test("actions 场景1/3: 在线播放·入库 | 下载·DLsite·ASMR One", () => {
  assert.deepEqual(
    resolveActions(input({ source: "asmr", mode: "asmr", hasOnline: true, hasRj: true })),
    ["online-play", "import", "asmr-download", "open-dlsite", "open-asmr"]
  );
});

test("actions 场景1/3 已入库: 入库 → 本地详情", () => {
  assert.deepEqual(
    resolveActions(
      input({ source: "asmr", mode: "asmr", hasOnline: true, hasLocalWork: true, hasRj: true })
    ),
    ["online-play", "go-local", "asmr-download", "open-dlsite", "open-asmr"]
  );
});

// 场景 2：DLsite 页，asmr.one 不存在 → 不显示 在线播放 / ASMR One 下载 / ASMR One 外链
test("actions 场景2: 入库·DLsite（无在线播放、无 asmr 下载、无 ASMR One 外链）", () => {
  assert.deepEqual(
    resolveActions(input({ source: "dlsite", mode: "dlsite", hasOnline: false, hasRj: true })),
    ["import", "open-dlsite"]
  );
});

// 场景 4：作品库，本地有音轨，asmr.one 存在
test("actions 场景4: 播放·下载·扫描音轨·关联本地·LLM翻译·在线播放·ASMR One下载·DLsite页面·ASMR One·删除", () => {
  assert.deepEqual(
    resolveActions(
      input({ source: "work", mode: "local", hasOnline: true, hasLocalWork: true, hasRj: true })
    ),
    [
      "play-all",
      "dl-download",
      "scan-tracks",
      "bind-local",
      "translate",
      "online-play",
      "asmr-download",
      "open-dlsite",
      "open-asmr",
      "delete",
    ]
  );
});

// 场景 4b：作品库，本地有音轨，asmr.one 不存在
test("actions 场景4b: 同场景4 去掉 在线播放 / ASMR One下载", () => {
  assert.deepEqual(
    resolveActions(
      input({ source: "work", mode: "local", hasOnline: false, hasLocalWork: true, hasRj: true })
    ),
    [
      "play-all",
      "dl-download",
      "scan-tracks",
      "bind-local",
      "translate",
      "open-dlsite",
      "open-asmr",
      "delete",
    ]
  );
});

// 场景 5：作品库，本地无音轨，asmr.one 不存在（无「播放」）
test("actions 场景5: 下载·扫描音轨·关联本地·LLM翻译·DLsite页面·ASMR One·删除", () => {
  assert.deepEqual(
    resolveActions(input({ source: "work", mode: "dlsite", hasOnline: false, hasRj: true })),
    [
      "dl-download",
      "scan-tracks",
      "bind-local",
      "translate",
      "open-dlsite",
      "open-asmr",
      "delete",
    ]
  );
});

// 场景 6：作品库，本地无音轨，asmr.one 存在（无「播放」）
test("actions 场景6: 在线播放·ASMR One下载·下载·扫描音轨·关联本地·LLM翻译·DLsite页面·ASMR One·删除", () => {
  assert.deepEqual(
    resolveActions(input({ source: "work", mode: "asmr", hasOnline: true, hasRj: true })),
    [
      "online-play",
      "asmr-download",
      "dl-download",
      "scan-tracks",
      "bind-local",
      "translate",
      "open-dlsite",
      "open-asmr",
      "delete",
    ]
  );
});

// 边界：作品库无 RJ（理论上不会发生），仍应有 播放/删除 等本地操作，且不显示 DLsite 外链
test("actions 边界: 作品库无 RJ 时不显示 DLsite 外链", () => {
  const keys = resolveActions(
    input({ source: "work", mode: "local", hasLocalWork: true, hasRj: false })
  );
  assert.ok(!keys.includes("open-dlsite"));
  assert.ok(keys.includes("play-all"));
  assert.ok(keys.includes("delete"));
});

// 边界：作品库页永不显示 入库 / 本地详情（自身即本地详情页）
test("actions 边界: 作品库页永不出现 入库 / 本地详情", () => {
  for (const hasLocalWork of [true, false]) {
    const keys = resolveActions(input({ source: "work", mode: "local", hasLocalWork }));
    assert.ok(!keys.includes("import"));
    assert.ok(!keys.includes("go-local"));
  }
});

// 边界：DLsite 页无 RJ 时不显示 入库（无 RJ 无法入库）与 DLsite 外链
test("actions 边界: DLsite 无 RJ 时无 入库 / DLsite 外链", () => {
  assert.deepEqual(
    resolveActions(input({ source: "dlsite", mode: "dlsite", hasOnline: false, hasRj: false })),
    []
  );
});

// 边界：作品库本地无音轨但 asmr.one 不存在时，仍有 关联本地（唯一的补救入口）
test("actions 边界: 场景5 必须保留 关联本地", () => {
  assert.ok(
    resolveActions(input({ source: "work", mode: "dlsite", hasOnline: false })).includes("bind-local")
  );
});

// ============ 按钮文案与样式 ============
test("label: asmr 下载按钮在作品库内叫「ASMR One下载」", () => {
  assert.equal(actionLabel("asmr-download", "work"), "ASMR One下载");
});

test("label: asmr 下载按钮在 ASMR ONE / DLsite 页内叫「下载」", () => {
  assert.equal(actionLabel("asmr-download", "asmr"), "下载");
  assert.equal(actionLabel("asmr-download", "dlsite"), "下载");
});

test("label: 在线播放（原「在线试听」改名）", () => {
  assert.equal(actionLabel("online-play", "work"), "在线播放");
});

test("label: DLsite 外链在作品库页叫「DLsite 页面」", () => {
  assert.equal(actionLabel("open-dlsite", "work"), "DLsite 页面");
  assert.equal(actionLabel("open-dlsite", "dlsite"), "DLsite");
});

test("variant: 首个按钮为 primary", () => {
  assert.equal(actionVariant("online-play", 0), "primary");
  assert.equal(actionVariant("play-all", 0), "primary");
});

test("variant: 外链与本地关联/删除为 ghost，其余为 outline", () => {
  assert.equal(actionVariant("open-dlsite", 5), "ghost");
  assert.equal(actionVariant("open-asmr", 6), "ghost");
  assert.equal(actionVariant("bind-local", 3), "ghost");
  assert.equal(actionVariant("delete", 9), "ghost");
  assert.equal(actionVariant("dl-download", 1), "outline");
  assert.equal(actionVariant("asmr-download", 6), "outline");
});

// ============ 在线标签转只读 Tag ============
test("tagsFromNames: 转成只读 Tag，id 为负数避免与本地标签冲突", () => {
  assert.deepEqual(tagsFromNames(["耳かき", "添い寝"]), [
    { id: -1, name: "耳かき", color: "#888888", is_preset: false },
    { id: -2, name: "添い寝", color: "#888888", is_preset: false },
  ]);
});

test("tagsFromNames: 空数组返回空数组", () => {
  assert.deepEqual(tagsFromNames([]), []);
});

// ============ 状态角标（spec §7） ============
// clickable 由调用方决定：仅「非作品库页 + 已入库」时可点击跳本地详情
// （作品库页自身就是本地详情，再跳自己是无意义的死交互）。
test("badge: 已下载 → 绿", () => {
  assert.deepEqual(resolveStatusBadge("downloaded", true, true), {
    text: "已下载",
    tone: "green",
    clickable: true,
  });
});

test("badge: 下载中 → 强调色", () => {
  assert.deepEqual(resolveStatusBadge("downloading", true, true), {
    text: "下载中",
    tone: "accent",
    clickable: true,
  });
});

test("badge: 已入库未下载 → 未下载（中性）", () => {
  assert.deepEqual(resolveStatusBadge("not_downloaded", true, true), {
    text: "未下载",
    tone: "neutral",
    clickable: true,
  });
});

test("badge: 作品库页（clickable=false）时角标不可点击", () => {
  assert.equal(resolveStatusBadge("downloaded", true, false).clickable, false);
});

test("badge: 未入库 → 未入库（黄），不可点击（避免死链接）", () => {
  assert.deepEqual(resolveStatusBadge("not_downloaded", false, false), {
    text: "未入库",
    tone: "yellow",
    clickable: false,
  });
});