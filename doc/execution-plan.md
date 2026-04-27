# 执行文档

本文件用于指导实际开发。版本目标参考 [版本路线图](./roadmap.md)，安全边界参考 [安全策略](./safety-policy.md)。

## 当前执行目标

当前从 V0 和 V1 开始：

- V0：完成工程初始化、页面框架、规则系统、Native Core 基础结构。
- V1：完成只读扫描、安全清理、操作日志和基础回滚。

## 里程碑计划

| 里程碑 | 目标 | 预计产出 |
|---|---|---|
| M0 | 项目初始化 | 可启动桌面应用、基础页面、命令调用链路 |
| M1 | 规则系统 | 三类规则文件、规则读取、路径变量解析 |
| M2 | 扫描系统 | 磁盘概览、目录扫描、风险分类、进度事件 |
| M3 | 安全清理 | 临时文件、回收站、缓存清理、结果报告 |
| M4 | 日志回滚 | 操作日志、失败记录、基础回滚数据结构 |
| M5 | 用户目录迁移 | Shell Folder 读取、迁移预检、迁移执行 |
| M6 | 开发工具管理 | DevSpace 页面、工具识别、标准缓存迁移 |
| M7 | 打包发布 | exe 安装包、zip 便携版、发布说明 |

## M0 验收

- 应用可以本地启动。
- Windows 桌面窗口可以打开。
- 页面能在 Dashboard、Cleanup、Migration、DevSpace、Analysis、Logs 间切换。
- 前端可以调用 Rust command。
- Rust 后端 `cargo check` 通过。
- 前端 `npm run build` 通过。

## 每次开发前需要读取哪些文档

| 开发内容 | 必读 | 可选 |
|---|---|---|
| 页面和交互 | `architecture.md`、`execution-plan.md` | `product-plan.md` |
| 扫描和清理 | `safety-policy.md`、`execution-plan.md` | `roadmap.md` |
| 用户目录迁移 | `safety-policy.md`、`execution-plan.md` | `architecture.md` |
| 开发工具迁移 | `devspace-manager.md`、`safety-policy.md` | `roadmap.md` |
| 打包发布 | `architecture.md`、`roadmap.md` | `execution-plan.md` |
