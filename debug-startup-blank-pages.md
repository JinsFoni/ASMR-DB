[OPEN]

# 调试记录：startup-blank-pages

## 症状
- 无法启动（白屏窗口）
- 启动后所有页面（首页/导入/设置/详情/下载管理）内容区空白，但有侧边栏
- 用户双击项目目录里的 exe

## 期望
- 应用可正常启动
- 各菜单/选项进入后页面正常渲染

## 复现步骤
1. 双击 exe 启动（用户操作，非沙箱）
2. 出现窗口，有侧边栏，内容区全白

## 环境信息
- OS: Windows 10/11
- 安装方式：直接运行项目目录下的 exe（target/debug 或 target/release）
- 项目内存在多个构建：debug exe (18:44) / release exe (19:14 新构建)

## 假设
- H1(Rust/DB)：`db::init_database()` 失败（disk I/O error）导致 setup 中断
  - 结果：❌ 已被证伪为沙箱假象。TRAE 沙箱拦截了对 `dlsite_manager.db-shm/-wal` 的写操作（日志见 Sandbox Error）。数据库文件本身完好（复制后打开正常、30 个对象、表结构齐全）。真实环境无此拦截。
- H2(前端路由)：懒加载 chunk 加载失败 / 前端运行时错误导致内容区空白
  - 结果：❌ 用户确认双击的是 debug 目录 exe；且侧边栏可见说明主包加载成功，真正的白屏来自 debug exe 加载 devUrl 失败。
- H3(构建来源)：用户双击的是 debug 构建 exe，Tauri debug 模式默认从 `devUrl: http://localhost:1420` 加载前端；dev server 未运行时前端资源 404 → 白屏
  - 证据：tauri.conf.json 的 devUrl=http://localhost:1420；debug exe 二进制内包含 "localhost:1420" 且不含内嵌 assets；release exe 包含 "/assets/index-"（内嵌 dist 资源）；netstat 显示 1420 端口无监听；用户确认双击的是 debug 目录 exe
  - 结果：✅ 确认。正确用法是运行 release 构建的 exe（内嵌资源，不依赖 dev server）

## 第二阶段：导入/筛选/删除/本地文件问题（用户反馈）
- H4(导入失败)：`upsert_work` 对 FTS5 虚拟表执行 UPSERT（ON CONFLICT DO UPDATE）→ SQLite 报 "UPSERT not implemented for virtual table works_fts"
  - 证据：Python 复现 `OperationalError('UPSERT not implemented for virtual table "works_fts"')`；DB 定义 `CREATE VIRTUAL TABLE works_fts USING fts5(... content='works' ...)` 为外部内容表
  - 修复：✅ 改为先 DELETE 再 INSERT（实测 OK）
- H5(筛选页报错)：`list_works` 无搜索词时 base_sql="FROM works w"（无 WHERE），status/tag 过滤直接拼接 " AND EXISTS..." → `near "AND": syntax error`
  - 证据：Python 复现 `near "AND": syntax error`；用户报错信息完全吻合
  - 修复：✅ 增加 has_where 判断，无 WHERE 时用 WHERE 拼接（实测 OK）
- H6(本地文件绑定失败/删除不掉)：`local_files` 表无 UNIQUE(work_id, local_path) 约束，但 `upsert_local_file` 用 ON CONFLICT(work_id, local_path) → "ON CONFLICT clause does not match any PRIMARY KEY or UNIQUE constraint"
  - 证据：Python 复现该错误；用户库 local_files 为空（从未成功绑定）
  - 修复：✅ migrate 增加去重 + CREATE UNIQUE INDEX（实测 OK）
- H7(删除报错)：works_fts 为 external content 表（content='works'），对其执行 DELETE FROM works_fts 报 "database disk image is malformed" → 删除作品失败
  - 证据：Python 复现 `DatabaseError('database disk image is malformed')`
  - 修复：✅ 重建为独立 FTS5 表（不绑定 content），migrate 中 DROP+CREATE+回填（实测 DELETE/INSERT 均 OK）

## 证据记录
- pre-fix（沙箱内运行 release exe）：
  - `[DEBUG] run() entered` → `setup() start` → `db::init_database() begin` → `db::init_database() err: disk I/O error`
  - 无任何前端日志（前端未加载成功）
  - 沙箱报错：Not allow operate files: dlsite_manager.db-shm / -wal
- 数据库文件完整性：复制到工作区后 sqlite 打开正常，journal_mode=wal，30 个对象
- APPDATA 数据目录（真实环境）：dlsite_manager.db (131072B) / .db-shm / .db-wal（含用户导入的 3 个作品）
- 项目内 exe 列表：
  - target/debug/deps/dlsite_asmr_manager.exe (18:44, 26MB, 含 localhost:1420, 无内嵌资源)
  - target/release/dlsite-asmr-manager.exe (20:02 重建, 含内嵌 assets + 修复)
- post-fix: 待用户真实环境验证

## 结论
- root cause（白屏）：用户双击 debug exe，其从 devUrl(localhost:1420) 加载前端，dev server 未运行 → 白屏。应运行 release exe。
- root cause（导入失败）：FTS5 虚拟表不支持 UPSERT。
- root cause（筛选报错）：list_works SQL 拼接缺 WHERE。
- root cause（本地文件/删除）：local_files 缺唯一约束 + works_fts 外部内容表 DELETE 损坏。
- fix：见上 H4-H7，已重建 release exe。

## 下一步
1. 用户双击新 release exe 验证：能正常启动、首页/导入/设置/下载/详情均正常
2. 导入 RJ01113387 不再报 UPSERT 错误
3. 已下载/未下载筛选页不再报语法错误
4. 本地文件夹绑定成功（local_files 写入）、删除作品成功
5. 验证通过后清理插桩与调试产物
