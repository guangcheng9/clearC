# 开发工具空间管理

## 背景

很多 AI 编辑器、开发工具、SDK、包管理器和自动化工具会把配置、缓存、模型、下载产物放在用户目录下。

示例：

```txt
C:\Users\当前用户\.android
C:\Users\当前用户\.aws
C:\Users\当前用户\.bun
C:\Users\当前用户\.cache
C:\Users\当前用户\.cargo
C:\Users\当前用户\.chromium-browser-snapshots
C:\Users\当前用户\.claude
C:\Users\当前用户\.agents
C:\Users\当前用户\.apifox-mcp-server
C:\Users\当前用户\.codebuddy
```

## 页面能力

开发工具瘦身页面展示：

- 工具名称。
- 当前路径。
- 占用空间。
- 数据类型。
- 风险等级。
- 推荐动作。
- 迁移方式。
- 是否支持回滚。

## 迁移策略

| 类型 | 推荐动作 |
|---|---|
| 可重新生成的缓存 | 清理或迁移 |
| 支持官方配置的工具 | 配置或环境变量迁移 |
| 不支持配置但适合迁移 | Junction |
| 含 token、key、secret、credential、auth | 只读扫描和提示 |

## V1/V2 优先支持

- npm、pnpm、bun 缓存。
- cargo、rustup。
- Gradle、Android。
- Playwright、Puppeteer 浏览器缓存。
- 通用 `.cache`。

`.aws`、云服务凭据、AI 工具账号授权目录默认不自动迁移。
