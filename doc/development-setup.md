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
