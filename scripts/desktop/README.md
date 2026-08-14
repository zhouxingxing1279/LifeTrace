# Windows 桌面应用一键部署

该脚本复用 `.github/workflows/release-windows.yml`，不会在本机重复实现签名和发布逻辑。

## 前置条件

- Windows
- 已安装 `git`
- 已安装 GitHub CLI `gh`
- `gh auth login` 已登录，并对 `zhouxingxing1279/LifeTrace` 有 Actions 权限
- 发布模式要求当前仓库工作区干净

## 发布正式 Windows 安装包

在仓库根目录运行：

```bat
deploy-desktop.cmd
```

脚本会：

1. 校验当前仓库和 `origin`；
2. 切换并 fast-forward 同步 `main`；
3. 校验 `package.json`、`tauri.conf.json`、`Cargo.toml` 版本一致；
4. 若同版本 Release 已存在则拒绝覆盖，要求先提升版本号；
5. 触发 `Release Windows Installer` GitHub Actions；
6. 等待 CI 构建、Tauri 签名、NSIS 发布完成；
7. 校验 `LifeTrace-Releases` 中的 Release 和 `latest.json`。

如果已经手动同步到干净的 `main`，可跳过拉取：

```bat
deploy-desktop.cmd -NoSyncMain
```

## 发布后立即安装

```bat
deploy-desktop.cmd -Mode PublishAndInstall
```

静默安装：

```bat
deploy-desktop.cmd -Mode PublishAndInstall -SilentInstall
```

## 仅安装最新正式版本

不会重新发布：

```bat
deploy-desktop.cmd -Mode InstallLatest
```

默认安装包下载到临时目录并在成功安装后删除。保留安装包：

```bat
deploy-desktop.cmd -Mode InstallLatest -KeepInstaller
```

指定下载目录：

```bat
deploy-desktop.cmd -Mode InstallLatest -OutputDirectory D:\Downloads\LifeTrace
```

## 版本规则

正式发布不允许覆盖已有版本。例如 `v0.2.1` 已存在时，脚本会停止，而不是覆盖旧的签名安装包和 updater manifest。先同时更新以下三个版本字段，再重新运行：

- `apps/desktop/package.json`
- `apps/desktop/src-tauri/tauri.conf.json`
- `apps/desktop/src-tauri/Cargo.toml`
