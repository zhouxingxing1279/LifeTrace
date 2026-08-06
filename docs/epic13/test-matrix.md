# EPIC-13 测试矩阵

## 1. 自动化门禁

| ID | 层级 | 场景 | 命令/检查 | 预期 |
|---|---|---|---|---|
| A-01 | TypeScript | Web、桌面和共享类型静态检查 | `npm run lint` | 0 error |
| A-02 | Unit | 金额精确转换与非法输入 | `npm run test:unit` | 整数分，无浮点误差 |
| A-03 | Unit | 财务/笔记/英语 Schema 工厂 | `npm run test:unit` | 必填字段完整 |
| A-04 | Unit | Cookie 登录与完整 scope | `npm run test:unit` | credentials=include，scope 完整 |
| A-05 | Unit | 云端 snapshot 初始加载 | `npm run test:unit` | 获取 cursor 和实体；无 outbox |
| A-06 | Unit | push accepted | `npm run test:unit` | 服务器确认后才更新内存状态 |
| A-07 | Unit | push 失败 | `npm run test:unit` | 内存状态不变，错误明确 |
| A-08 | Unit | 版本冲突 | `npm run test:unit` | 应用服务器实体并记录冲突 |
| A-09 | Unit | CSV 导入和重复检测 | `npm run test:unit` | candidate 状态、疑似重复可识别 |
| A-10 | Unit | 全局搜索 | `npm run test:unit` | 财务/笔记/英语跨模块命中 |
| A-11 | Regression | 现有 Tauri Web 构建 | `npm run web:build` | 成功 |
| A-12 | Build | 独立 PWA 构建 | `npm run pwa:build` | 成功生成 `dist-web` |
| A-13 | Artifact | PWA 关键文件 | CI shell checks | index/manifest/sw/icons 存在 |
| A-14 | Performance | 构建产物预算 | `du -sk dist-web` | 小于 5 MB |
| A-15 | Integration | Browser Cookie + CSRF sync | PostgreSQL CI | snapshot/pull/push 成功 |

## 2. 云端直写行为

| ID | 场景 | 操作 | 预期 |
|---|---|---|---|
| C-01 | 初次登录 | 登录后进入概览 | 从 `/sync/snapshot` 加载数据 |
| C-02 | 新增记录 | 新增账单/笔记/生词 | `/sync/push` accepted 后才显示 |
| C-03 | 请求失败 | 断网或服务器 5xx 时保存 | 显示未保存，表单内容不清空 |
| C-04 | 浏览器刷新 | 刷新页面 | 重新恢复 Session 并获取 snapshot |
| C-05 | 增量刷新 | 点击“刷新云端” | 按 cursor 调用 `/sync/pull` |
| C-06 | 冲突 | 两端修改同一记录 | Web 显示服务器最新版本与冲突提示 |
| C-07 | 删除 | 删除云端实体 | accepted 后从页面移除 |
| C-08 | 批量导入 | 上传 CSV/XLSX | 去重后按批 push，逐条统计结果 |

## 3. 浏览器本地数据检查

使用 DevTools 验收：

- IndexedDB 中没有 LifeTrace 业务数据库；
- localStorage/sessionStorage 中没有财务、笔记、英语实体或 outbox；
- Cookie 为 HttpOnly，JavaScript 无法读取；
- Service Worker Cache Storage 不保留页面或 API 响应；
- 退出登录后 React 页面状态被清空；
- 公共设备模式的会话有效期由服务器策略控制。

## 4. 功能联调

### 财务

- 快速记录收入与支出；
- 新建、重命名、删除账户；
- 新建和删除自定义分类；
- 设置月度预算并计算使用进度；
- 微信/支付宝/银行 CSV 或 XLSX 导入；
- candidate 账单确认或忽略；
- 相同交易单号不重复上传。

### 笔记

- Tiptap 富文本编辑；
- 富文本切换 Markdown；
- 文件夹筛选；
- 标签创建与关联；
- 置顶、编辑和删除；
- 多端冲突显示服务器版本；
- 附件按钮明确提示 EPIC-12 依赖，不声称上传成功。

### 英语

- 浏览云端只读文章；
- 选择正文并保存高亮；
- 提交阅读总结；
- 添加、掌握和删除生词；
- 查看阅读时间、生词和高亮统计。

### 设备与安全

- 查看设备和活动会话；
- 重命名设备；
- 撤销非当前设备和会话；
- 隐私模式遮罩金额；
- 退出后不能继续访问页面内存数据。

## 5. 响应式验收

| 视口 | 预期 |
|---|---|
| 360×800 | 底部导航可用；表单单列；无横向阻断 |
| 768×1024 | 平板布局可用；卡片两列或单列自适应 |
| 1366×768 | 完整侧栏和多列仪表盘 |
| 1920×1080 | 内容宽度合理，不产生大面积无意义留白 |

## 6. PWA 验收

- Manifest 可解析；
- Chrome/Edge 可添加到主屏幕；
- 快速记账、笔记、英语和搜索快捷入口有效；
- 新 Service Worker 安装后出现更新提示；
- 断网首次打开显示浏览器网络错误或应用联网提示；
- Service Worker 不提供离线壳，也不缓存 API。

## 7. 合并门禁

只有以下条件同时满足才允许合入：

- EPIC13 Web PWA 检查通过；
- EPIC-03 PostgreSQL 检查通过；
- EPIC-05 Windows Sync 回归通过；
- PR 无未解决阻断审查；
- PR head SHA 与最终检查 SHA 一致；
- 合入后 `main` 的对应检查继续通过。
