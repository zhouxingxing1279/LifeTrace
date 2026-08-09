# LifeTrace 本地加密相册执行方案

> 状态：待实施  
> 目标分支：`main`  
> 适用模块：现有相册功能  
> 核心原则：私密相册只在本机运行、只在本机存储、只在本机加解密，不向任何云端或远程服务上传任何私密数据。

## 1. 项目目标

在 LifeTrace 现有相册功能中增加“私密相册”。该功能不是普通的页面密码锁，而是一个真实的本地加密存储系统。

必须达到以下目标：

1. 私密相册只能通过独立密码解锁。
2. 没有正确密码时，无法获得解密主密钥。
3. 即使直接复制数据库、应用目录和加密文件，也无法查看照片或视频。
4. 不提供密码找回、恢复密钥、管理员重置或开发者后门。
5. 忘记密码后，私密相册中的数据永久无法恢复，只能删除整个私密相册。
6. 原图、视频、缩略图、文件名、EXIF、标签、备注和子相册名称全部加密。
7. Electron Renderer 不得持有主密钥，不得直接访问私密文件。
8. 锁定状态下不得暴露照片数量、封面、文件名、相册名、访问记录或搜索结果。
9. 私密相册不参与现有照片同步功能。
10. 私密相册不调用任何云存储、远程备份、远程 AI、远程缩略图或远程分析接口。

准确的安全表述：

> 私密相册在锁定和静态存储状态下，没有正确密码无法解密。密码遗失后，照片和视频永久不可恢复。

---

## 2. 本地化硬性边界

私密相册必须遵守以下本地化规则：

- 所有原图、视频、缩略图、元数据和密钥只保存在本机。
- 所有加密与解密操作只在本机 Electron 主进程或本地安全服务中执行。
- 私密相册数据不得进入现有手机照片同步流程。
- 私密相册数据不得上传到 iCloud、OneDrive、Google Drive、对象存储或自建服务器。
- 私密相册数据不得通过 HTTP、WebSocket、WebRTC、FTP 或其他网络协议发送。
- 私密照片不得调用云端 AI 分类、OCR、人脸识别、内容审核或图像增强服务。
- 私密缩略图不得使用远程 URL。
- 私密相册不得生成可被浏览器或其他设备访问的局域网分享链接。
- 私密相册不得出现在二维码分享、手机上传页面或远程访问接口中。
- 应用日志和崩溃报告不得包含私密文件内容、原始文件名、路径、EXIF 或预览数据。

实现时应在代码层增加显式隔离：

```text
普通相册资源 -> 可参与本地同步或用户允许的现有流程
私密相册资源 -> 永远只允许 local-only storage
```

任何资源一旦移入私密相册，必须移除其普通同步任务、远程队列、分享链接和普通搜索索引。

---

## 3. 功能范围

### 3.1 第一版必须实现

- 创建私密相册
- 设置独立密码
- 解锁、主动锁定和自动锁定
- 普通照片或视频移入私密相册
- 私密照片或视频移回普通相册
- 私密照片浏览和视频播放
- 私密子相册
- 私密回收站
- 修改密码，且必须输入旧密码
- 永久删除私密相册
- 导入、移出、删除过程的事务保护
- 应用异常退出后的恢复和临时文件清理
- 密文完整性校验
- 本地离线完整性检查

### 3.2 第一版明确不实现

- 密码找回
- 恢复密钥
- 邮件、短信或安全问题重置
- 管理员重置
- 开发者万能密码
- Windows Hello、指纹、人脸或系统 PIN 解锁
- 自动登录或记住密码
- 云端备份
- 云同步
- 多设备同步
- 局域网同步
- 远程访问
- 分享链接
- 私密照片二维码分享
- 云端 AI 分析
- 云端 OCR 或人脸识别
- 输错多次自动删除数据

---

## 4. 用户交互设计

### 4.1 导航入口

在现有相册左侧导航增加：

```text
全部照片
相册
最近导入
已收藏
私密相册 🔒
回收站
```

锁定状态只显示“私密相册”和锁图标，不显示：

- 照片数量
- 子相册数量
- 存储空间
- 最后访问时间
- 相册封面
- 最近查看内容

### 4.2 首次创建

首次进入时显示创建向导：

1. 介绍本地加密相册。
2. 明确说明不提供密码找回。
3. 明确说明所有数据只保存在当前电脑。
4. 输入密码。
5. 再次确认密码。
6. 勾选不可恢复确认项。
7. 创建随机主密钥和本地加密目录。

固定警告文案：

> 私密相册只保存在当前电脑，不会上传到任何云端。系统不提供密码找回、恢复密钥或管理员重置功能。忘记密码后，其中所有照片和视频将永久无法恢复。

密码要求：

- 至少 12 个字符
- 禁止纯 4 位或 6 位 PIN
- 显示密码强度
- 两次输入必须一致

### 4.3 解锁页面

只包含：

- 密码输入框
- 显示或隐藏密码按钮
- 解锁按钮
- 永久删除私密相册入口
- 不可恢复提示

不得出现“忘记密码”“联系客服”“恢复密钥”等入口。

### 4.4 自动锁定

默认空闲 5 分钟锁定，可配置：

- 窗口失焦立即锁定
- 1 分钟
- 5 分钟
- 10 分钟
- 30 分钟

以下事件必须立即锁定：

- 应用退出
- 用户主动锁定
- 系统锁屏
- 系统休眠
- 用户账户切换
- 主窗口关闭
- Renderer 崩溃
- 本地服务异常

---

## 5. 安全架构

```text
Electron Renderer
        │
        │ 受控 IPC
        ▼
Electron Main Process / 本地安全服务
        ├── VaultSessionManager
        ├── VaultKeyManager
        ├── VaultCryptoService
        ├── VaultFileService
        ├── VaultRepository
        ├── VaultThumbnailService
        ├── VaultMigrationService
        └── VaultRecoveryService
                │
                ▼
           本机加密存储
```

职责边界：

- Renderer 只负责界面。
- Renderer 不保存密码派生密钥或主密钥。
- Renderer 不得直接读取私密文件路径。
- 所有加解密操作在主进程或本地安全服务中完成。
- 所有私密接口默认禁止网络访问。
- 私密资源类型必须带有 `storageScope = local_private` 或等价标志。
- 同步服务必须显式拒绝 `local_private` 资源。

---

## 6. 密钥设计

采用两级密钥：

```text
用户密码
   ↓ Argon2id
密码派生密钥 KEK
   ↓ AES-256-GCM 解密
随机主密钥 MEK
   ↓
加密照片、视频、缩略图和元数据
```

### 6.1 初始化流程

1. 使用安全随机数生成 256 位主密钥 MEK。
2. 生成随机 salt。
3. 使用 Argon2id 从用户密码派生 KEK。
4. 使用 AES-256-GCM 和 KEK 加密 MEK。
5. 保存加密后的 MEK、salt、nonce、authentication tag 和 Argon2id 参数。
6. 不保存明文密码、明文 KEK、明文 MEK 或备用密钥。

### 6.2 解锁流程

1. 用户输入密码。
2. 根据本地保存的 KDF 参数派生 KEK。
3. 尝试认证解密 MEK。
4. 认证成功后创建短生命周期会话。
5. MEK 只存在于主进程内存。

### 6.3 修改密码

1. 使用旧密码派生旧 KEK。
2. 解密 MEK。
3. 使用新密码派生新 KEK。
4. 使用新 KEK 重新加密 MEK。
5. 原子替换密钥记录。
6. 验证新密码可以解锁后删除旧记录。

没有旧密码时不得修改密码，只能永久删除整个私密相册。

### 6.4 算法要求

- 密码派生：Argon2id
- 文件和元数据加密：AES-256-GCM
- 随机数：Node.js `crypto.randomBytes` 或等价 CSPRNG
- 每个对象独立随机 nonce
- Argon2id 目标派生耗时 300～800 毫秒
- 初始内存成本至少 64 MiB，并在目标电脑上进行基准测试

禁止：

- 密码直接作为 AES 密钥
- 固定 nonce
- 多文件复用 nonce
- MD5 或 SHA-1 派生密码
- 密钥写入日志、LocalStorage、IndexedDB 或配置文件

---

## 7. 本地存储设计

建议目录：

```text
app-data/
  gallery/
    public/
  vault/
    config/
    objects/
    thumbnails/
    database/
    temp/
    quarantine/
```

加密文件采用随机 ID：

```text
vault/objects/4a/4a8d3f81fbd74fc1.vlt
vault/objects/b2/b284d90c813543cf.vlt
```

路径中不得出现原始文件名、日期、相册名、人物、标签或地理位置。

### 7.1 加密对象格式

定义版本化容器：

```text
Magic
FormatVersion
Algorithm
NonceLength
TagLength
EncryptedMetadataLength
EncryptedContentLength
Nonce
EncryptedMetadata
EncryptedContent
AuthenticationTag
```

原文件名、MIME 类型、尺寸、时长、拍摄时间和 EXIF 必须位于加密元数据中。

### 7.2 缩略图

缩略图必须单独加密：

1. 内存中读取原图。
2. 内存中生成缩略图。
3. 立即加密缩略图。
4. 仅保存密文。
5. 清理明文 Buffer。

查看时由主进程解密，通过短生命周期 token 或受控 IPC 提供给 Renderer。页面关闭、锁定或会话失效时立即释放 Blob URL。

### 7.3 元数据

以下信息全部加密：

- 原文件名与扩展名
- MIME 类型
- 图片尺寸与视频时长
- 拍摄时间与导入时间
- EXIF 与 GPS
- 标签、备注与收藏状态
- 子相册名称
- 排序信息
- 来源设备与原始路径

---

## 8. 数据库建议

```sql
CREATE TABLE vault_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    format_version INTEGER NOT NULL,
    kdf_algorithm TEXT NOT NULL,
    kdf_salt BLOB NOT NULL,
    kdf_memory_cost INTEGER NOT NULL,
    kdf_time_cost INTEGER NOT NULL,
    kdf_parallelism INTEGER NOT NULL,
    encrypted_master_key BLOB NOT NULL,
    master_key_nonce BLOB NOT NULL,
    master_key_auth_tag BLOB NOT NULL,
    auto_lock_seconds INTEGER NOT NULL DEFAULT 300,
    failed_attempts INTEGER NOT NULL DEFAULT 0,
    next_attempt_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE vault_assets (
    id TEXT PRIMARY KEY,
    object_path TEXT NOT NULL UNIQUE,
    thumbnail_path TEXT,
    encrypted_metadata BLOB NOT NULL,
    metadata_nonce BLOB NOT NULL,
    metadata_auth_tag BLOB NOT NULL,
    encrypted_size INTEGER NOT NULL,
    format_version INTEGER NOT NULL,
    content_fingerprint TEXT,
    state TEXT NOT NULL DEFAULT 'active',
    storage_scope TEXT NOT NULL DEFAULT 'local_private',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE vault_albums (
    id TEXT PRIMARY KEY,
    encrypted_name BLOB NOT NULL,
    name_nonce BLOB NOT NULL,
    name_auth_tag BLOB NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE vault_album_assets (
    album_id TEXT NOT NULL,
    asset_id TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (album_id, asset_id)
);

CREATE TABLE vault_operations (
    id TEXT PRIMARY KEY,
    operation_type TEXT NOT NULL,
    asset_id TEXT,
    source_path TEXT,
    target_path TEXT,
    state TEXT NOT NULL,
    error_code TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

`storage_scope = local_private` 必须作为同步层的硬拒绝条件，而不是仅依赖 UI 不显示同步按钮。

---

## 9. 核心服务

### `VaultKeyManager`

负责主密钥生成、密码派生、MEK 包装、解锁、修改密码和内存清理。

```ts
interface VaultKeyManager {
  initialize(password: string): Promise<void>;
  unlock(password: string): Promise<void>;
  lock(): void;
  changePassword(oldPassword: string, newPassword: string): Promise<void>;
  isUnlocked(): boolean;
  withMasterKey<T>(handler: (key: Buffer) => Promise<T>): Promise<T>;
}
```

禁止提供长期返回主密钥的接口。

### `VaultSessionManager`

负责短生命周期会话、最后活动时间、自动锁定、IPC 会话验证和系统生命周期事件。

### `VaultCryptoService`

负责流式文件加解密、元数据加解密、缩略图加解密、GCM 完整性认证和格式兼容。视频和大文件必须流式处理。

### `VaultFileService`

负责目录初始化、`.partial` 文件、原子写入、`fsync`、临时文件清理、损坏对象隔离。

标准写入：

```text
写入 .partial
  ↓
完成加密
  ↓
完整性验证
  ↓
fsync
  ↓
原子 rename
```

### `VaultRepository`

负责私密资源、子相册、回收站、事务状态和数据库迁移。

### `VaultThumbnailService`

负责在内存中生成缩略图、加密落盘、限制缓存容量并在锁定时清除。

### `VaultMigrationService`

负责普通相册与私密相册之间的本机迁移，不得调用同步服务或任何网络接口。

### `VaultRecoveryService`

负责启动时清理临时文件、恢复未完成事务、隔离损坏密文和检查文件引用一致性。

---

## 10. IPC 安全

建议接口：

```text
vault.initialize
vault.unlock
vault.lock
vault.status
vault.listAssets
vault.getThumbnail
vault.getPreview
vault.importAssets
vault.moveToPublic
vault.moveToTrash
vault.restoreFromTrash
vault.deletePermanently
vault.createAlbum
vault.renameAlbum
vault.changePassword
vault.deleteVault
```

每个请求必须校验：

- sessionId 存在且未过期
- 私密相册已解锁
- 参数类型与长度
- 路径位于允许目录
- 资源属于私密相册
- 操作状态无冲突
- 请求大小和并发限制

Renderer 不得传入任意绝对路径。

预览建议使用 `vault-preview://` 自定义协议，token 必须与 sessionId 和 assetId 绑定，短期有效，锁定后全部失效，响应禁止进入普通 HTTP 缓存。

---

## 11. 移入私密相册

```text
选择普通资源
  ↓
取消并删除该资源的待执行同步任务
  ↓
创建本地迁移事务
  ↓
读取普通原文件
  ↓
提取元数据并生成缩略图
  ↓
加密原文件、缩略图和元数据
  ↓
写入私密数据库
  ↓
验证密文可以认证解密
  ↓
删除普通数据库记录
  ↓
删除普通原文件和普通缩略图
  ↓
清理普通搜索索引、缓存、分享记录和同步记录
  ↓
提交事务
```

必须先完成加密、持久化和校验，才能删除明文原文件。

失败时：

- 不删除普通原文件
- 清理未完成的 `.partial`
- 标记事务失败
- 下次启动恢复或回滚
- 不允许出现两边均不可用的状态

---

## 12. 移回普通相册

```text
验证私密会话
  ↓
创建移出事务
  ↓
解密到受控本地临时文件
  ↓
校验完整性
  ↓
原子移动到普通相册目录
  ↓
生成普通缩略图
  ↓
写入普通数据库
  ↓
删除私密数据库记录和密文
  ↓
清理临时文件
  ↓
提交事务
```

移回普通相册后，资源是否参与现有同步功能由普通相册规则决定；移回之前绝不能参与任何同步。

---

## 13. 删除与回收站

私密照片删除后进入独立私密回收站：

- 只有解锁后可访问
- 不出现在普通回收站
- 不参与全局搜索或统计
- 文件继续保持加密
- 支持恢复和永久删除

删除整个私密相册时：

1. 使当前会话失效。
2. 优先删除加密主密钥记录。
3. 删除密文原图、视频和缩略图。
4. 删除私密数据库。
5. 删除临时文件和缓存。
6. 删除事务与预览 token。

禁止因多次输错密码自动删除数据。

---

## 14. 防暴力破解

建议延迟策略：

```text
第 1～3 次：无额外延迟
第 4 次：等待 5 秒
第 5 次：等待 15 秒
第 6 次：等待 30 秒
后续逐步增加
最高等待 5 分钟
```

错误提示统一为：

> 密码错误或私密相册数据无法验证。

不能区分密码错误、认证标签错误或主密钥损坏。

---

## 15. 内存、缓存与生命周期

锁定时主进程必须：

```ts
masterKeyBuffer.fill(0);
sessionStore.clear();
previewTokenStore.clear();
decryptedMetadataCache.clear();
thumbnailMemoryCache.clear();
```

Renderer 必须：

- revoke 所有 Blob URL
- 清空资源列表和当前预览
- 清空私密搜索条件
- 清空状态管理中的私密数据
- 返回锁定页面

必须检查并限制：

- Chromium HTTP Cache
- GPUCache
- LocalStorage
- IndexedDB
- Service Worker Cache
- Session Storage
- 崩溃日志
- Windows 最近文件
- 应用临时目录

私密预览不得进入普通缓存。生产环境应禁止私密页面使用 DevTools。

---

## 16. 网络隔离实现要求

私密相册必须有可测试的网络隔离，而不是只写在产品说明中。

### 16.1 数据模型隔离

所有私密资源必须标记：

```text
storageScope = local_private
syncEligible = false
shareEligible = false
remoteAnalysisEligible = false
```

### 16.2 同步层硬拒绝

普通同步服务入口必须增加校验：

```ts
if (asset.storageScope === 'local_private') {
  throw new Error('VAULT_ASSET_REMOTE_ACCESS_FORBIDDEN');
}
```

### 16.3 网络请求隔离

- Vault 模块不得导入云端存储 SDK。
- Vault 模块不得调用普通上传服务。
- Vault 模块不得生成远程 URL。
- Vault 预览只允许 `vault-preview://` 或本地内存数据。
- 测试环境应拦截网络请求并验证私密操作期间请求数为零。

### 16.4 日志和遥测

如应用存在遥测系统，私密模块只能上报非内容型状态，例如匿名错误码和耗时；第一版建议完全不为私密模块上报遥测。

---

## 17. 分阶段执行计划

### 阶段一：现有相册架构审计

任务：

1. 定位普通照片数据库表。
2. 定位原文件、缩略图和缓存目录。
3. 定位现有照片同步、上传、分享和远程访问入口。
4. 定位搜索索引和首页统计。
5. 定位 Electron 主进程与 Renderer 边界。
6. 输出一张照片从导入到展示、同步的完整数据流。

交付物：

```text
docs/local-encrypted-album/01-gallery-architecture-audit.md
```

### 阶段二：安全与本地隔离设计

任务：

1. 固定密钥结构和算法版本。
2. 定义加密文件格式。
3. 定义数据库迁移。
4. 定义 IPC 和会话模型。
5. 定义同步硬拒绝规则。
6. 定义威胁模型和错误码。

交付物：

```text
docs/local-encrypted-album/02-security-design.md
docs/local-encrypted-album/03-storage-format.md
docs/local-encrypted-album/04-threat-model.md
docs/local-encrypted-album/05-api-design.md
```

### 阶段三：加密基础模块

任务：

- Argon2id 密钥派生
- MEK 生成、包装和解包
- 流式文件加解密
- 元数据和缩略图加解密
- GCM 认证
- 格式版本识别
- 单元测试

### 阶段四：Vault 后端服务

实现：

- `VaultKeyManager`
- `VaultSessionManager`
- `VaultCryptoService`
- `VaultFileService`
- `VaultRepository`
- `VaultThumbnailService`
- `VaultMigrationService`
- `VaultRecoveryService`
- 系统锁屏、休眠和退出事件

### 阶段五：创建、解锁与锁定 UI

完成最小闭环，并验证：

- 重启后必须重新输入密码
- 绕过 Renderer 页面仍无法读取密文
- 数据目录不存在明文照片
- 解锁过程无网络请求

### 阶段六：移入与移出

实现单张、批量、视频迁移、进度、中断恢复和回滚。移入时必须清理同步队列、分享记录、普通索引和缓存。

### 阶段七：浏览、子相册与回收站

实现私密瀑布流、预览、视频播放、子相册、搜索、备注和私密回收站。全部只在解锁状态运行。

### 阶段八：修改密码与永久删除

实现旧密码验证、MEK 重新包装、原子更新和无密码永久删除流程。

### 阶段九：安全与网络隔离测试

覆盖密文篡改、密码错误、缓存泄露、崩溃恢复、大文件、并发迁移和零网络请求测试。

---

## 18. 测试方案

### 18.1 密码测试

- 正确密码解锁
- 错误密码拒绝
- 重启后重新验证
- 修改密码后旧密码失效
- 没有旧密码无法修改
- 删除后无法恢复

### 18.2 加密测试

- 同一文件加密两次产生不同密文
- 修改密文、nonce 或 tag 后认证失败
- 错误主密钥无法解密
- 大视频流式加解密
- 磁盘空间不足和中途退出回滚

### 18.3 泄露测试

锁定状态检查：

- 普通数据库
- 私密数据库
- 文件目录
- 缩略图目录
- Electron Cache 和 GPUCache
- LocalStorage 与 IndexedDB
- 日志和崩溃报告
- Windows 最近文件
- 临时目录
- 搜索索引和首页统计

不得发现可直接查看的私密照片、缩略图、文件名、标签、相册名称或 EXIF。

### 18.4 网络隔离测试

在创建、解锁、浏览、搜索、移入、移出、删除和修改密码时：

- 拦截所有 HTTP、HTTPS、WebSocket 和 WebRTC 请求。
- 验证 Vault 操作产生的远程请求数为零。
- 验证私密资源传入同步服务时被硬拒绝。
- 验证私密资源无法生成分享链接或局域网访问地址。
- 验证应用断网时私密相册全部核心功能正常可用。

### 18.5 生命周期测试

- 应用退出
- 窗口关闭
- 系统锁屏
- 系统休眠
- Renderer 崩溃
- 主进程异常退出
- 强制关机
- 应用更新重启

---

## 19. 推荐代码目录

```text
src/
  main/
    vault/
      VaultService.ts
      VaultKeyManager.ts
      VaultSessionManager.ts
      VaultCryptoService.ts
      VaultFileService.ts
      VaultRepository.ts
      VaultThumbnailService.ts
      VaultMigrationService.ts
      VaultRecoveryService.ts
      vaultIpcHandlers.ts
      vaultErrors.ts
      vaultTypes.ts
      vaultConstants.ts
  renderer/
    modules/
      vault/
        pages/
        components/
        stores/
        api/
  shared/
    vault/
      types.ts
      schemas.ts
      errorCodes.ts

tests/
  unit/vault/
  integration/vault/
  security/vault/
  fixtures/vault/
```

---

## 20. 完成定义

只有全部满足以下条件，功能才视为完成：

1. 使用真实文件加密，而不是仅做 UI 鉴权。
2. 没有正确密码无法获得 MEK。
3. 不存在密码找回、恢复密钥或后门。
4. 原图、视频、缩略图和敏感元数据全部加密。
5. 主密钥不进入 Renderer。
6. 重启、锁屏、休眠和退出后重新锁定。
7. 普通相册中不存在私密照片明文副本。
8. 普通搜索、统计和回收站不泄露私密信息。
9. 迁移操作具有事务、校验和中断恢复。
10. 密文被篡改后必须认证失败。
11. 修改密码必须输入旧密码。
12. 忘记密码时只能永久删除。
13. 私密资源不进入同步队列。
14. 私密资源不能生成分享链接或远程 URL。
15. 所有私密操作在断网环境中正常工作。
16. 网络拦截测试确认远程请求数为零。
17. 单元测试、集成测试和安全测试全部通过。
18. 打包版本完成缓存与泄露检查。

---

## 21. 推荐实施顺序

```text
现有相册与同步链路审计
  ↓
安全设计和本地隔离规则
  ↓
加密基础模块
  ↓
密钥与会话管理
  ↓
创建、解锁与锁定
  ↓
单张资源移入和预览
  ↓
批量迁移与中断恢复
  ↓
移出、回收站与删除
  ↓
修改密码
  ↓
网络隔离与缓存安全测试
  ↓
完整回归和发布
```

不要先开发复杂 UI，再补加密。必须先完成可独立测试的加密核心、本地隔离和迁移事务，再接入相册页面。