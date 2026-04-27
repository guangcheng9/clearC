# ClearC 文档入口

ClearC 是一个面向 Windows 的系统盘清理与迁移桌面工具。产品原则是：先扫描、再确认、可回滚，不把释放空间建立在不可控删除之上。

## 文档分工

后续修改不需要每次读取全部文档。按任务类型读取对应文档即可：

| 任务 | 优先阅读 |
|---|---|
| 调整产品范围、功能取舍 | [产品方案](./product-plan.md)、[版本路线图](./roadmap.md) |
| 调整技术栈、目录结构、打包方式 | [项目架构](./architecture.md) |
| 涉及删除、迁移、回滚、权限 | [安全策略](./safety-policy.md) |
| 涉及 AI 工具、开发工具、SDK 缓存 | [开发工具空间管理](./devspace-manager.md) |
| 确认每个版本做什么 | [版本路线图](./roadmap.md) |
| 开始编码、排期、验收 | [执行文档](./execution-plan.md) |
| 用户怎么操作、结果如何判断 | [用户操作手册](./user-manual.md) |
| 本地环境、启动、构建 | [开发环境](./development-setup.md) |
| 打包发布、产物说明 | [发布说明](./release.md) |
| V2 版本设计 | [V2 版本规划](./v2-plan.md) |

## 文档维护规则

- 产品能力只在 `product-plan.md` 定义，其他文档只引用。
- 版本节奏只在 `roadmap.md` 维护。
- 安全边界只在 `safety-policy.md` 维护。
- 开发工具迁移策略只在 `devspace-manager.md` 维护。
- 执行细节只在 `execution-plan.md` 维护。
- 用户可见的功能、按钮流程、正确/错误结果必须同步 `user-manual.md`。
