# LifeTrace Windows 发布与在线更新

本文档说明如何发布 LifeTrace Windows 安装包，并让已安装的应用通过 Tauri 2
Updater 在线更新。

## 架构

```text
私有源码仓库 LifeTrace
    ↓ GitHub Actions 构建（推送 v* 标签或手动触发）
公开仓库 LifeTrace-Releases
    ↓ 发布 NSIS 安装包、.sig 签名、latest.json
已安装的 LifeTrace
    ↓ Tauri Updater 检查和下载
自动安装新版本并重启
```

- 源码仓库：`zhouxingxing1279/LifeTrace`（私有）
- 发布仓库：`zhouxingxing1279/LifeTrace-Releases`（公开，只放安装包、签名、更新 JSON 和发行说明）
- 更新端点：`https://github.com/zhouxingxing1279/LifeTrace-Releases/releases/latest/download/latest.json`

## 一、生成签名密钥（只需一次）

在 PowerShell 中执行：

```powershell
New-Item -ItemType Directory -Force "$HOME\.tauri"

npx tauri signer generate `
  -w "$HOME\.tauri\lifetrace-updater.key"
```

生成两个文件：

- `lifetrace-updater.key`：**私钥**，绝不能提交到 Git、不能放入 `.env` 并提交。
- `lifetrace-updater.key.pub`：**公钥**，可以公开，用于填入客户端配置。

### 私钥与公钥的用途

- **公钥**：把 `lifetrace-updater.key.pub` 的**完整内容**填入
  `src-tauri/tauri.conf.json` 的 `plugins.updater.pubkey` 字段，替换占位符
  `REPLACE_WITH_TAURI_UPDATER_PUBLIC_KEY`。注意必须是密钥内容本身，不能填文件路径。
- **私钥**：把 `lifetrace-updater.key` 的**完整内容**存入 GitHub Secret
  `TAURI_SIGNING_PRIVATE_KEY`。
- **密码**：生成密钥时如果设置了密码，把密码存入 GitHub Secret
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。

### 安全要求

- 必须安全备份私钥（离线/密码管理器均可）。
- 丢失私钥后，旧安装版本无法验证由新密钥签名的更新，只能让用户手动重新安装一次。
- 不要把私钥放入 `.env` 并提交。
- 项目 `.gitignore` 已忽略 `*.key`、`*.key.pub` 和 `.tauri/`。

## 二、配置 Release Token（只需一次）

发布工作流需要向公开仓库 `zhouxingxing1279/LifeTrace-Releases` 写入 Release，因此
不能使用源码仓库默认的 `GITHUB_TOKEN`（它没有权限写另一个仓库）。

1. 打开 GitHub → Settings → Developer settings → Personal access tokens →
   Fine-grained tokens → Generate new token。
2. Repository access 只选择 `zhouxingxing1279/LifeTrace-Releases`。
3. Permissions 至少需要：
   - **Metadata: Read**
   - **Contents: Read and write**
4. 复制 Token，添加到源码仓库 `zhouxingxing1279/LifeTrace` 的
   Settings → Secrets and variables → Actions，名称：
   `RELEASE_TOKEN`。
5. 不要把 Token 写入客户端代码或工作流明文。

公开的 `LifeTrace-Releases` 仓库只存放安装包、`.sig` 签名、`latest.json` 和发行说明，
不需要公开源码。

## 三、首次发布（桥接版本）

当前版本 `0.2.1` 没有 Updater，无法在线升级。第一个带 Updater 的版本（建议
`0.2.2`）需要用户**手动下载安装一次**；安装后，后续版本才能通过应用内在线更新。

首次发布的完整命令：

```powershell
# 1. 统一修改版本号
# package.json
# src-tauri/tauri.conf.json
# src-tauri/Cargo.toml
# 三处必须一致，例如 0.2.2

# 2. 同步 Cargo.lock
cargo check --manifest-path src-tauri/Cargo.toml

# 3. 执行检查
npm test
node scripts/check-version-consistency.mjs

# 4. 提交代码
git add .
git commit -m "release: LifeTrace v0.2.2"

# 5. 创建并推送标签（标签版本必须与配置版本一致）
git tag v0.2.2
git push origin main
git push origin v0.2.2
```

推送 `v0.2.2` 标签后，`.github/workflows/release-windows.yml` 会自动：

1. 在 `windows-latest` 上安装 Node 22 与 Rust stable。
2. 执行 `npm ci`、版本一致性检查、lint、单元测试和前端构建。
3. 构建 NSIS 安装包，并用 `TAURI_SIGNING_PRIVATE_KEY` 生成 `.sig` 签名。
4. 在 `zhouxingxing1279/LifeTrace-Releases` 创建**正式（非 Draft）** Release，
   上传安装包、`.sig` 和 `latest.json`。
5. 下载并校验 `latest.json` 的版本、`windows-x86_64` 平台条目、安装包 URL 和签名。

也可以到 Actions 页面手动触发（Workflow dispatch），输入与配置一致的版本号即可。

### latest.json 格式

工作流生成的 `latest.json` 形如：

```json
{
  "version": "0.2.2",
  "notes": "更新说明",
  "pub_date": "RFC 3339 时间",
  "platforms": {
    "windows-x86_64": {
      "signature": "安装包签名",
      "url": "https://github.com/zhouxingxing1279/LifeTrace-Releases/releases/download/v0.2.2/LifeTrace_0.2.2_x64-setup.exe"
    }
  }
}
```

平台键名由 Tauri 构建产物自动确认（Windows x64 为 `windows-x86_64`），不要手工硬编码
其他平台键。

## 四、后续版本发布

重复“三、首次发布”的步骤：修改三个版本号 → `cargo check` → 测试与一致性检查 →
提交 → 打标签 → 推送。已安装 0.2.2 的用户会收到更新弹窗，自动下载、校验签名并安装。

## 注意事项

- Release 不能保持 Draft，否则 `releases/latest` 无法作为更新端点。
- 不要覆盖或删除旧的 Release，否则旧版本可能无法正常升级。
- 每次发布前必须确认三个文件版本一致：
  `package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml`。
- `latest.json` 中的 `signature` 必须是 `.sig` 文件内容，URL 必须指向公开
  `LifeTrace-Releases` 的安装包。
- 发布工作流需要的 Secrets：
  - `RELEASE_TOKEN`（Fine-grained PAT，仅授权 `LifeTrace-Releases`，Metadata Read + Contents Read/Write）
  - `TAURI_SIGNING_PRIVATE_KEY`（私钥完整内容）
  - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`（私钥密码）
