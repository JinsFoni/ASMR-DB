# 🎧 DLsite ASMR Manager v2.0.1

一个轻量、美观、便携的 DLsite 作品管理桌面端，聚焦 **ASMR 音声作品** 的管理与播放，同时支持同人游戏、漫画等作品类型。

基于 [设计书.md](设计书.md) 迭代升级。

---

## ✨ 核心功能

| 模块 | 说明 |
|:---|:------|
| 🗂️ **作品库** | 网格 / 列表视图切换、FTS5 全文毫秒级搜索、已下载/未下载状态筛选、作品分组（Group）、标签筛选、多字段排序、分页 |
| ➕ **导入作品** | 输入 RJ 号自动抓取元数据（封面/多语言标题/社团/声优/标签/简介）；**智能双数据源**：优先 DLsite 官方，受阻自动无缝回退 asmr.one；**导入时可选一键关联本地文件夹并智能匹配分组**；本地文件夹递归扫描智能识别 RJ 号 |
| 📄 **作品详情** | 多语言标题展示、元数据与声优信息、自定义标签编辑与着色、音轨列表、一键复制 RJ 号、官方页面一键跳转、**一键关联本地文件夹并自动划分分组** |
| ⬇️ **下载引擎** | 下载队列管理、HTTP Range 断点续传、实时下载进度与速率监控、暂停/继续/取消、下载后自动解压 `.zip` 并自动扫描提取音轨；**支持 asmr.one 完整文件树自定义勾选下载**（音频/字幕/图片快速筛选） |
| 🎵 **音频播放器** | 播放/暂停/切歌、进度条拖拽 Seek、0.5x~2.0x 倍速播放、循环模式（单曲/列表/不循环）、播放进度持久化记忆、常驻迷你控制栏 |
| 💬 **智能字幕系统** | 支持 `.vtt` / `.srt` / `.lrc` 字幕解析；**asmr.one 在线试听时自动抓取字幕并优先匹配中文字幕**；支持**独立桌面透明悬浮歌词小窗口**（置顶、可拖拽） |
| 🌐 **asmr.one 在线试听** | 内置虚拟协议代理与鉴权流式播放，无需下载即可在线畅听并同步显示歌词字幕 |
| 💾 **便携独立数据库与数据迁移** | **SQLite 数据库直接存放在软件同级目录**（`dlsite_manager.db`），解压即用，即拷即走；设置页支持基于 `VACUUM INTO` 的**一键导出备份与安全还原** |
| 🎨 **三套个性化主题** | 包含 **深色模式 (Midnight Obsidian)**、**浅色明亮 (Clean Slate)** 与 **樱花粉系 (Sakura Pink)**，实时一键切换 |
| ⚙️ **设置面板** | 三套主题风格切换、自定义下载目录选择、asmr.one Token 配置与内置登录抓取、数据库备份/还原与目录一键定位 |

---

## 🛠️ 技术栈

- **桌面框架**: [Tauri 2.0](https://tauri.app/) (Rust)
- **前端**: Vue 3 + TypeScript + Vite + Tailwind CSS + Pinia + Lucide Icons
- **数据库**: SQLite (`rusqlite`, WAL 模式 + FTS5 独立全文索引)
- **网络/下载**: `reqwest` (Rust, 支持 Range 分块与流式请求)
- **音频引擎**: Howler.js (Web Audio, 支持自定义 URI scheme 代理与 `convertFileSrc` 加载本地文件)

---

## 🚀 运行与构建

### 前置要求

| 工具 | 版本 | 说明 |
|:---|:---|:------|
| [Rust](https://rustup.rs/) | 1.75+ | MSVC toolchain |
| [Node.js](https://nodejs.org/) | 18+ | 含 npm |
| Visual Studio Build Tools | 2022 | 勾选 “使用 C++ 的桌面开发” 工作负载 |
| WebView2 | Win10 / Win11 自带 | 无需额外安装 |

### 本地开发模式

```powershell
npm install          # 安装前端依赖
npm run tauri dev    # 启动开发模式（支持前端热更新与 Rust 调试）
```

### 构建发布包

```powershell
npm run tauri build  # 生成 .msi / .exe 安装包与绿色版独立程序
```

构建输出路径位于：
- **绿色独立版 EXE**: `src-tauri/target/release/dlsite-asmr-manager.exe`
- **安装程序 (Setup EXE)**: `src-tauri/target/release/bundle/nsis/DLsite ASMR Manager_2.0.0_x64-setup.exe`
- **MSI 安装包**: `src-tauri/target/release/bundle/msi/DLsite ASMR Manager_2.0.0_x64_en-US.msi`

---

## 📝 使用指南

### 1. 导入作品
* **RJ 号导入**：在侧边栏点击 **导入作品**，输入 RJ 号（例如 `RJ01014447`），可选择 DLsite 或 asmr.one 数据源预览元数据并导入。
* **本地文件夹扫描**：选择存放 ASMR 文件夹的根目录，程序将自动递归扫描并识别 RJ 号，同时可将分类文件夹智能设为作品分组（Smart Group）。

### 2. asmr.one 在线试听与字幕
* 在「设置 → asmr.one」中粘贴 Token 或使用内置登录窗口登录。
* 在作品详情页点击 **asmr.one 在线试听**，系统会自动检索该作品的在线音轨并**自动抓取匹配的 `.vtt` / `.srt` / `.lrc` 字幕**。
* 进入播放页即可边听边看字幕，点击顶部字幕栏还可开启**桌面悬浮歌词**。

### 3. 本地播放与音轨管理
* 在作品详情页点击 **扫描音轨** 或 **播放**，即可加载本地音频。
* 支持快捷键：
  - `Space`：播放 / 暂停
  - `←` / `→`：快退 / 快进 5 秒
  - `↑` / `↓`：音量微调

---

## 📂 便携数据存储说明

* 数据库文件 `dlsite_manager.db`（及其 WAL 索引）默认直接保存在 **`dlsite-asmr-manager.exe` 所在的同一目录下**。
* **数据迁移/备份**：只需复制软件目录下的 `dlsite_manager.db` 文件（若存在 `-wal` 和 `-shm` 一并复制）即可完整迁移作品库、播放记录、标签和设置。

---

## 📄 设计书

完整设计规范见 [设计书.md](设计书.md)（含数据库表结构、API 协议规范、UI/UX 设计稿）。

---
> 由 小玥喵 🐱 制作 · DLsite ASMR Manager v2.0.0
