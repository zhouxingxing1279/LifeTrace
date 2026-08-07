# LifeTrace Desktop

LifeTrace 应用端，包括 Tauri 桌面程序、React UI、本地 SQLite、本地照片/私密相册能力、局域网服务以及浏览器版本。

## 安装

```powershell
npm ci
```

## 开发

```powershell
npm run dev
```

浏览器版本：

```powershell
npm run browser:dev
```

## 验证

```powershell
npm run lint
npm run test:unit
npm run web:build
npm run browser:build
npm run test:rust
```

## 构建 Windows 应用

```powershell
npm run build
```

应用端依赖仓库根目录的共享 Rust crates（`../../crates`）以及生成契约（`../../contracts`），但不依赖 `services/cloud` 的内部实现。与云端通信只通过公开 API/同步协议完成。
