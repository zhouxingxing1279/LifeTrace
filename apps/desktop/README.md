# LifeTrace Desktop

LifeTrace 的 Windows 桌面端基于 Tauri 2 + WebView2，前端使用 React/Vite，后端使用 Rust + SQLite。

## 启动行为

桌面窗口创建后会先由静态 HTML 渲染启动界面，再进入 React/Tauri 初始化流程。窗口状态、主题、桥接层和本地 SQLite 服务初始化均受统一错误兜底保护；任何启动异常都应显示“LifeTrace 启动失败”和具体错误，而不是留下空白 WebView。

Windows 构建使用 ES2020 作为桌面 WebView 的 JavaScript 输出目标，以兼容较旧的 WebView2 Runtime。

故障判断：能看到启动/失败页面，说明 WebView2 已加载本地 HTML，问题位于 JavaScript 或 Tauri 初始化；如果连静态启动页都完全不可见，则优先检查 WebView2 Runtime 或安装包资源加载。

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
