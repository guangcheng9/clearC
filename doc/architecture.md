# 项目架构

## 推荐技术栈

```txt
Tauri 2 + React + TypeScript + Rust + JSON/SQLite
```

## 架构模型

ClearC 不需要服务器。这里的“前端”和“后端”是桌面应用内部的分层：

```txt
ClearC 桌面程序
├─ UI 层：React / TypeScript
├─ Native Core：Rust / Tauri Commands
├─ 本地配置：JSON / SQLite
└─ Windows 能力：文件系统、注册表、回收站、权限、环境变量
```

## UI 层职责

- 首页仪表盘。
- 清理项列表。
- 扫描进度。
- 迁移向导。
- 风险提示。
- 大文件列表。
- 日志页面。
- 回滚入口。

UI 层不直接删除文件，也不直接修改注册表。

## Native Core 职责

- 扫描目录。
- 统计文件大小。
- 判断文件是否被占用。
- 清理临时文件。
- 清空回收站。
- 读取和修改 Windows Shell Folder。
- 设置环境变量。
- 创建 Junction。
- 执行迁移。
- 写入日志。
- 执行回滚。

## 当前目录结构

```txt
clearC/
  src/
    pages/
    App.tsx
    App.css
  src-tauri/
    src/
      commands/
      core/
      storage/
      lib.rs
      main.rs
  rules/
    cleanup.rules.json
    migration.rules.json
    devspace.rules.json
  doc/
```

## 打包形式

- `ClearC_Setup.exe`：正式安装版。
- `ClearC_Portable.zip`：绿色便携版。
- `ClearC.msi`：企业部署版，后续支持。
