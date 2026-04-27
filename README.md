# ClearC

ClearC 是一个面向 Windows 的系统盘清理与迁移桌面工具。当前项目采用 Tauri 2 + React + TypeScript + Rust 架构。

## 当前状态

- 已完成项目初始化。
- 已建立基础页面框架。
- 已建立 Rust Native Core command 骨架。
- 已建立 `rules/` 清理、迁移、开发工具规则目录。
- 已建立 `doc/` 产品、架构、安全、路线图和执行文档。

## 常用命令

```powershell
npm install
npm run build
```

Rust 后端检查：

```powershell
cd src-tauri
cargo check
```

桌面开发模式：

```powershell
npm run tauri dev
```

## 文档入口

查看 [doc/README.md](./doc/README.md)。
