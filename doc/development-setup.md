# 开发环境

## 当前环境

```txt
Node.js: v22.18.0
npm: 10.9.3
Rust: stable-x86_64-pc-windows-msvc
rustc: 1.95.0
```

Rust stable 通过以下命令安装：

```powershell
rustup default stable
```

## 初始化方式

项目通过 Tauri 官方脚手架初始化：

```powershell
npm create tauri-app@latest . -- --template react-ts --manager npm --tauri-version 2 --identifier com.clearc.app --yes --force
npm install
```

注意：`--force` 会覆盖当前目录内容。后续如果已有重要文件，禁止直接使用该参数。

## 常用命令

```powershell
npm run build
cargo check
npm run tauri dev
```

`cargo check` 需要在 `src-tauri` 目录下执行。

桌面开发和打包也可以使用：

```powershell
npm run tauri:dev
npm run tauri:build
```

## 协作流程

每次开发任务完成后，需要在回复中说明：

- 当前执行位置，例如 `V0 / M0` 或 `V1 / M2`。
- 完成内容。
- 验证命令和结果。
- 同步了哪些文档。
- 是否有待确认问题。

每次代码改动如果影响文档内容，需要同步更新 `doc/` 下对应文档。文档读取范围按 [文档入口](./README.md) 中的分工执行，不需要每次全量读取所有文档。

如果改动影响用户可见功能、按钮流程、结果展示、错误提示或安全提示，必须同步 [用户操作手册](./user-manual.md)。

## 当前初始化状态

- Tauri 2 + React + TypeScript 项目已建立。
- ClearC 应用名称、窗口标题、包名已更新。
- 前端页面骨架已建立。
- Rust command 模块已建立。
- `rules/` 规则目录已建立。
- `doc/` 文档目录已恢复。

## 验证结果

```txt
npm run build: passed
cargo check: passed
```

## 当前执行进度

- `V0 / M0`：已完成。
- `V0 / M1`：规则系统已接入，前端页面可展示规则文件数据。
- `V1 / M2`：基础只读扫描已完成，已实现系统盘概览、路径变量解析和规则路径统计。
- `V1 / M3`：安全清理进入隔离清理阶段，已实现清理预览、确认模型、计划草稿记录和 `%TEMP%` 隔离移动，不开放永久删除。
- `V1 / M4`：日志回滚基础已接入，隔离清理可通过 Logs 页面按操作记录回滚。
- `V1 / M5`：用户目录迁移进入只读检测和预检查阶段，已读取 Shell Folder 当前路径，并可评估目标盘、空间和路径冲突。
- `V1 / M6`：开发工具管理进入只读扫描阶段，已支持规则驱动统计开发工具、AI 工具和缓存目录占用。
- `V1 / M7`：打包发布准备已完成基础验证，release exe 可生成，NSIS/MSI 安装包因下载 NSIS 工具链超时暂未生成。
- `V1.1 / 可用性修复`：扫描、预览、隔离清理、迁移预检查和回滚已改为后台执行，并补充按钮 loading 与禁用原因提示。
- `V2 / M8-M9`：用户目录迁移执行与回滚代码加固已完成，支持跨盘移动 fallback、注册表类型保留、失败日志、前端确认短语和 Logs 页面迁移回滚入口；待真实用户目录手动验收。

## 最近验证结果

```txt
npm run build: passed
cargo check: passed
cargo fmt --check: passed
npm run tauri:build: exe passed, bundle failed on NSIS download timeout
npm run tauri:build -- --no-bundle: passed
cargo test: passed
```

最近一次 `V2 / M8-M9` 加固验证：

```txt
npm run build: passed
cargo check: passed
cargo fmt --check: passed
cargo test: passed, 5 tests
```

最近一次 `V2 / M8` 页面白屏修复：

```txt
问题：迁移计划确认短语输入框在 onChange 的状态更新回调内读取 event.currentTarget.value，输入第一个字符时可能触发运行时异常并导致白屏。
修复：先同步读取输入值，再更新 confirmations 状态；新增 ErrorBoundary，后续页面渲染异常显示错误面板而不是整页白屏。
npm run build: passed
cargo check: passed
cargo fmt --check: passed
```
