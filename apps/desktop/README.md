# LifeTrace Desktop

LifeTrace 的 Windows 桌面端基于 Tauri 2 + WebView2，前端使用 React/Vite，后端使用 Rust + SQLite。

## 启动行为

窗口创建后先由静态 HTML 渲染启动界面，再进入 React/Tauri 初始化。窗口状态、主题、桥接层和本地 SQLite 初始化都受统一错误兜底保护；启动异常应显示“LifeTrace 启动失败”和具体错误，不能留下空白 WebView。

Windows 构建使用 ES2020 作为桌面 WebView 的 JavaScript 输出目标，以兼容较旧 WebView2 Runtime。

故障判断：若能看到启动或失败页面，说明 WebView2 已加载本地 HTML，问题位于 JavaScript/Tauri 初始化；若连静态启动页都不可见，则优先检查 WebView2 Runtime 或安装包资源。

## 开发

```bash
npm install
npm run desktop
```

## 校验

```bash
npm test
npm run test:rust
```
