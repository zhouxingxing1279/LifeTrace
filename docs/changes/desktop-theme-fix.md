# 桌面应用主题启动与同步修复

桌面应用此前同时存在云端 `appearance.theme`、本地 appPreferences 和 SQLite `settings.dark` 三条主题状态路径，导致设置页可以显示“深色”，但 Tauri 渲染层仍停留在浅色，重启后也可能无法恢复。

本次统一桌面主题应用路径：

- Tauri 首屏在 React 启动前读取 `lifetrace.app-preferences.v1` 和非敏感主题 Cookie，直接设置 `data-theme` 与首屏背景，避免浅色闪屏。
- 桌面云工作台在云端 `user.preference / appearance.theme` 加载完成后调用统一的 `setAppThemePreference`，让云端设置真正作用于桌面 DOM。
- 本地工作台的 SQLite `settings.dark` 通过 Zustand 订阅桥接到同一主题服务，保留旧数据兼容性。
- 显式切换浅色/深色后会同时更新 DOM、appPreferences 和首屏主题提示，后续启动保持一致。
- 云端偏好仍是云端工作台的最终主题来源；本地缓存只负责桌面启动首屏和离线一致性。
