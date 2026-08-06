# EPIC-13 测试矩阵

## 1. 自动化门禁

| ID | 层级 | 场景 | 命令/检查 | 预期 |
|---|---|---|---|---|
| A-01 | TypeScript | Web、桌面和共享类型静态检查 | `npm run lint` | 0 error |
| A-02 | Unit | 金额精确转换与非法输入 | `npm run test:unit` | 整数分，无浮点误差 |
| A-03 | Unit | 财务/笔记/词汇 Schema 工厂 | `npm run test:unit` | 必填字段完整 |
| A-04 | Unit | Cookie 登录端点与 scope | `npm run test:unit` | 正确 URL、credentials、scope |
| A-05 | Unit | 离线 outbox 持久化 | `npm run test:unit` | 刷新仓库后数据仍存在 |
| A-06 | Unit | push accepted + pull | `npm run test:unit` | outbox 清空、cursor 和 version 更新 |
| A-07 | Unit | 版本冲突 | `npm run test:unit` | 采用服务器实体、产生冲突记录 |
| A-08 | Regression | 现有 Tauri Web 构建 | `npm run web:build` | 成功 |
| A-09 | Build | 独立 PWA 构建 | `npm run pwa:build` | 成功生成 `dist-web` |
| A-10 | Artifact | PWA 关键文件 | CI shell checks | index/manifest/sw/icons 存在 |
| A-11 | Performance | 构建产物预算 | `du -sk dist-web` | 小于 5 MB |

## 2. 认证与安全联调

| ID | 前置 | 步骤 | 预期 |
|---|---|---|---|
| M-01 | 有效账户 | 登录 | 进入概览；Cookie 不可由 JS 读取 |
| M-02 | 无效密码 | 登录 | 显示服务端稳定错误，不进入工作区 |
| M-03 | 已登录 | 刷新页面 | 通过 `/web/session` 恢复 |
| M-04 | 已登录 | 退出后刷新 | 无法进入工作区；本地用户缓存已清理 |
| M-05 | 公共设备 | 勾选公共设备登录 | session 标记 publicDevice，退出清理缓存 |
| M-06 | CSRF token 缺失 | 直接调用退出接口 | 服务端拒绝 |

## 3. 财务

| ID | 场景 | 预期 |
|---|---|---|
| F-01 | 创建现金账户 | 立即显示，进入 outbox，联网后获得 serverVersion |
| F-02 | 创建 23.50 元支出 | payload 为 `2350` 分，localDate 为本地自然日 |
| F-03 | 创建收入 | 列表使用收入样式，概览支出统计不计入 |
| F-04 | 删除流水 | 本地立即隐藏，服务端形成 tombstone |
| F-05 | 未指定账户记账 | 合法保存，不阻断记录 |

## 4. 笔记

| ID | 场景 | 预期 |
|---|---|---|
| N-01 | 创建无标题笔记 | 合法保存，列表显示“无标题笔记” |
| N-02 | 编辑正文 | 保留实体 ID，localVersion 增加，生成新 changeId |
| N-03 | 搜索 | 同时匹配标题和纯文本正文 |
| N-04 | 置顶/取消置顶 | 排序立即变化并进入同步队列 |
| N-05 | 删除 | 本地隐藏并同步 delete |

## 5. 英语

| ID | 场景 | 预期 |
|---|---|---|
| E-01 | 同步文章目录 | 文章只读显示，客户端不生成 article upsert |
| E-02 | 打开文章 | 显示正文，关闭后回到目录 |
| E-03 | 添加单词 | normalizedWord 小写，初始状态 LEARNING |
| E-04 | 切换掌握状态 | LEARNING/MASTERED 切换并同步 |

## 6. 离线、同步和冲突

| ID | 场景 | 预期 |
|---|---|---|
| S-01 | 在线首次进入 | 恢复 session 后自动同步 |
| S-02 | 断网后刷新 | 从 App Shell 与本地缓存进入工作区 |
| S-03 | 离线创建三类数据 | 即时显示，待同步计数增加 |
| S-04 | 恢复网络 | 自动 push 后 pull，待同步计数归零 |
| S-05 | 请求中断 | 原 change payload 保留，可按同一 changeId 重试 |
| S-06 | 两端修改同一笔记 | Web 采用服务器版本并显示冲突通知 |
| S-07 | 服务器删除实体 | pull 后本地实体移除 |
| S-08 | 多批数据 | 严格按 cursor 应用，整批成功后保存 cursor |

## 7. 兼容与体验

最低覆盖：

- Chrome/Edge 当前稳定版；
- Safari 当前稳定版；
- 360×800、390×844、768×1024、1440×900；
- 键盘 Tab 导航；
- `prefers-reduced-motion`；
- PWA standalone 模式和浏览器普通模式。
