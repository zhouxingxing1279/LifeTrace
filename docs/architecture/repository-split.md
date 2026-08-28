# LifeTrace Web / Desktop / Cloud 拆仓方案

## 目标

把当前 `LifeTraceManage/LifeTrace` 中的三端代码拆成三个独立代码仓：

- `LifeTraceManage/LifeTrace-web`
- `LifeTraceManage/LifeTrace-desktop`
- `LifeTraceManage/LifeTrace-cloud`

拆分完成并验证后，原 `LifeTrace` 不再作为 Web/Desktop/Cloud 的运行时代码依赖，可保留为项目入口/历史仓，或后续进一步清理归档。

## 当前耦合

当前 Monorepo 不能机械复制三个目录：

1. Desktop Rust 依赖：
   - `crates/lifetrace-contracts`
   - `crates/lifetrace-sync-client`
2. Desktop TypeScript 直接复用 Web：
   - `apps/web/src/app/AppContext`
   - `apps/web/src/app/DesktopFeatureRouter`
   - `apps/web/src/services/core`
   - `apps/web/src/styles/globals.css`
3. Cloud 依赖 `crates/lifetrace-contracts`。
4. Cloud Dockerfile 假设 `crates/` 与 `services/cloud/` 位于同一个 Monorepo。

## 拆分后的依赖关系

```text
LifeTrace-web
     ↑
     │ vendor/web
LifeTrace-desktop
     │ vendor/cloud
     ↓
LifeTrace-cloud
  ├── cloud server
  ├── crates/lifetrace-contracts
  ├── crates/lifetrace-sync-client
  └── contracts/
```

设计原则：

- Web 是浏览器前端源代码仓。
- Cloud 是后端与跨端协议/同步核心的唯一源码仓。
- Desktop 是原生桌面端代码仓；继续复用 Web feature layer，但依赖变成显式、可固定版本的 Git submodule。
- Desktop 的 Rust contracts/sync-client 从 Cloud 仓获取，不再依赖旧 Monorepo 相对路径。

## 自动拆仓脚本

仓库已加入：

```text
scripts/split-three-repos.ps1
```

Windows PowerShell 前置条件：

```powershell
git --version
gh --version
gh auth status
```

`gh` 登录账号需要具有 `LifeTraceManage` 组织创建仓库与 push 权限。

执行：

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\split-three-repos.ps1
```

脚本会按顺序：

1. Clone `LifeTraceManage/LifeTrace` main。
2. 用 `git subtree split` 提取 Web 历史并创建/push `LifeTrace-web`。
3. 用 `git subtree split` 提取 Cloud 历史。
4. 将 `lifetrace-contracts`、`lifetrace-sync-client`、生成 contracts 移交 Cloud 仓管理。
5. 修正 Cloud Cargo path 与 Dockerfile，创建独立 CI。
6. 创建/push `LifeTrace-cloud`。
7. 用 `git subtree split` 提取 Desktop 历史。
8. Desktop 添加：
   - `vendor/web -> LifeTrace-web`
   - `vendor/cloud -> LifeTrace-cloud`
9. 修正 Desktop Rust / TypeScript / Vite 的所有 Monorepo 相对路径。
10. 创建 Desktop 独立 CI 并 push `LifeTrace-desktop`。

## 为什么暂时不删除旧目录

第一次拆仓只负责建立新的三个仓库和 CI，不同时删除原目录。

必须先确认：

### Web

```powershell
npm install
npm run typecheck
npm test
npm run build
```

### Cloud

```powershell
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
docker build -t lifetrace-cloud .
```

### Desktop

Clone 时必须带 submodule：

```powershell
git clone --recurse-submodules https://github.com/LifeTraceManage/LifeTrace-desktop.git
```

然后：

```powershell
npm ci
npm run prepare:web-shared
npm run lint
npm run test:unit
npm run web:build
npm run test:rust
```

还必须人工回归：

- Desktop 启动正常。
- 本地 SQLite 正常。
- Cloud 登录/刷新 token 正常。
- Desktop 云工作台页面正常。
- Web/Cloud API 兼容。
- 同步功能正常。
- Windows 打包正常。

## 第二阶段：清理旧 Monorepo

只有三个新仓 CI 与人工回归全部通过后，才在原 `LifeTrace` 开独立 PR 删除：

```text
apps/web/
apps/desktop/
services/cloud/
```

同时处理：

- 删除/迁移旧三端 GitHub Actions。
- 更新根 README 与 docs 链接。
- 更新 release/deploy 脚本仓库地址。
- 更新本地开发说明。
- 检查所有 GitHub Actions secrets / environments。
- 检查部署机、Docker Compose、Caddy、更新服务器中的旧 repo 路径。

禁止把“拆仓”和“删除原代码”合并成一次不可回滚操作。

## 后续版本管理

建议三个仓独立版本：

- Web：`web-vX.Y.Z`
- Desktop：`desktop-vX.Y.Z`
- Cloud：`cloud-vX.Y.Z`

Desktop 的 `vendor/web` 与 `vendor/cloud` 指针就是它实际兼容的依赖版本，因此 Desktop 的构建是可重现的。

更新 Web/Cloud 依赖时，不直接追随 `main`，而是在 Desktop 仓显式更新 submodule commit，并通过 Desktop CI 后再合并。
