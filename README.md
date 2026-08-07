# LifeTrace

LifeTrace 是一个 local-first 个人管理平台。仓库采用 Monorepo：桌面/浏览器应用与云端服务位于同一个 Git 仓库，但拥有独立的源码目录、依赖、构建与部署流程。

## 仓库结构

```text
LifeTrace/
├─ apps/
│  └─ desktop/                 # Tauri + React + 本地 SQLite + 浏览器端
├─ services/
│  └─ cloud/                   # Rust + Axum + PostgreSQL 云端
├─ crates/                     # Desktop / Cloud 共用 Rust crates
├─ contracts/                  # 同步/API 契约与生成的类型
├─ deploy/                     # 云端部署配置
├─ design-system/              # 设计规范
├─ docs/                       # 项目文档
├─ scripts/                    # 仓库级脚本
└─ tools/                      # 契约生成等开发工具
```

## Desktop

安装依赖：

```powershell
npm ci --prefix apps/desktop
```

从仓库根目录启动桌面端：

```powershell
npm run dev
```

也可以直接进入应用目录：

```powershell
cd apps/desktop
npm run dev
```

常用命令：

```powershell
npm run lint
npm run test:unit
npm run web:build
npm run browser:build
npm run build:desktop
```

## Cloud

云端可以完全独立于 Desktop 构建和运行：

```powershell
npm run dev:cloud
npm run test:cloud
npm run build:cloud
```

Docker 与 PostgreSQL 开发配置位于 `deploy/cloud/`。

## 共享契约

Desktop 和 Cloud 不直接调用彼此内部代码；跨端边界通过同步/API 契约连接。共享协议代码位于：

- `crates/lifetrace-contracts/`
- `crates/lifetrace-sync-client/`
- `contracts/`

检查契约：

```powershell
npm run contracts:check
```

## 全量验证

```powershell
npm run test:all
```

## 只检出其中一部分

仓库仍然只有一个 `LifeTrace`，但可以使用 Git sparse-checkout 只取需要的子项目。

Desktop：

```bash
git clone --filter=blob:none --no-checkout https://github.com/zhouxingxing1279/LifeTrace.git
cd LifeTrace
git sparse-checkout init --cone
git sparse-checkout set apps/desktop crates contracts
git checkout main
```

Cloud：

```bash
git clone --filter=blob:none --no-checkout https://github.com/zhouxingxing1279/LifeTrace.git
cd LifeTrace
git sparse-checkout init --cone
git sparse-checkout set services/cloud crates contracts deploy/cloud scripts/cloud
git checkout main
```

## 发布边界

- Desktop：Windows/Tauri 安装包，独立版本与发布流程。
- Cloud：Rust 服务/Docker 镜像，独立部署。
- 两者通过 API/同步协议兼容，不要求运行时位于同一台机器，也不要求同时发布。
