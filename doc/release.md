# 发布说明

## 当前发布目标

当前阶段为 `V1 / M7：打包发布准备`，目标是验证 Windows 桌面应用可以构建，并形成安装包发布流程。

## 发布产物

Tauri 当前配置生成：

- NSIS 安装包：适合普通用户安装。
- MSI 安装包：适合 Windows 标准安装和后续企业部署。

便携版 `.zip` 不由 Tauri 默认直接生成。后续可以在 CI 或本地脚本中把构建后的可执行文件和必要资源打包为 `ClearC_Portable.zip`。

## 常用命令

前端构建：

```powershell
npm run build
```

Rust 检查：

```powershell
cd src-tauri
cargo check
```

桌面开发：

```powershell
npm run tauri:dev
```

生产打包：

```powershell
npm run tauri:build
```

## 产物位置

打包成功后，产物通常位于：

```txt
src-tauri/target/release/bundle/
```

release 可执行文件位于：

```txt
src-tauri/target/release/clearc.exe
```

## 发布前检查

每次发布前必须确认：

- `npm run build` 通过。
- `cargo check` 通过。
- `cargo fmt --check` 通过。
- `npm run tauri:build` 通过。
- `doc/` 文档已同步当前功能边界。
- 清理功能不执行永久删除。
- 迁移功能在未完成执行阶段前不修改注册表。
- 开发工具管理在未完成迁移阶段前不创建 Junction、不修改环境变量。

## 当前限制

- 当前真实清理仅支持 `%TEMP%` 隔离移动。
- 当前用户目录迁移仅支持只读检测和预检查。
- 当前开发工具管理仅支持只读扫描。
- 便携版 zip 需要后续补脚本生成。

## 2026-04-27 打包验证记录

执行：

```powershell
npm run tauri:build
npm run tauri:build -- --no-bundle
```

结果：

- 前端构建通过。
- Rust release 编译通过。
- 已生成 `src-tauri/target/release/clearc.exe`。
- NSIS/MSI bundling 阶段失败。
- `--no-bundle` 模式通过，可稳定生成 release exe。

失败原因：

```txt
Downloading https://github.com/tauri-apps/binary-releases/releases/download/nsis-3.11/nsis-3.11.zip
failed to bundle project `timeout: global`
```

判断：当前失败点是 Tauri bundler 下载 NSIS 工具链超时，不是应用代码编译失败。后续可以在网络稳定时重试，或预先安装/缓存 NSIS 打包工具链。
