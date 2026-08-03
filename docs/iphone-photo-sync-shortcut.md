# iPhone 照片同步快捷指令

LifeTrace 的照片同步客户端是 Apple“快捷指令”，不是 PWA，也不是 iOS 原生应用。它只在 iPhone 与运行 LifeTrace 的电脑处于同一局域网时工作，不经过 iCloud 下载照片，也不会持续驻留后台。

## 使用前准备

1. 在电脑上启动 LifeTrace，进入“照片”。
2. 确保 iPhone 与电脑连接同一个可信 Wi‑Fi。
3. Windows 首次询问防火墙权限时，仅允许“专用网络”，不要开放公网端口或配置路由器端口映射。
4. 在照片页面点击“导出 iPhone 信任证书”，把导出的 `LifeTrace-Local-CA.cer` 传到 iPhone，安装后前往“设置 → 通用 → 关于本机 → 证书信任设置”，为该证书启用完全信任。只需安装一次；电脑 IP 变化时 LifeTrace 会自动签发包含新 IP 的服务器证书。
5. 在 iPhone 的“快捷指令”中创建名为 `LifeTrace照片同步` 的快捷指令。名称需要完全一致，二维码才能通过 `shortcuts://run-shortcut` 调用它。

扫描电脑二维码后，需要轻点 iPhone 相机取景框底部出现的链接。Safari 随后会显示一个轻量确认页，再轻点“打开快捷指令”。该页面不是 PWA，不安装网页应用，也不保存长期令牌；它用于避免部分 iOS 版本拦截服务器直接跳转到 `shortcuts://`。

## 配置文件

快捷指令将敏感配置保存到以下位置之一：

```text
在我的 iPhone 上/Shortcuts/LifeTrace/photo-sync-config.json
```

也可以选择：

```text
iCloud Drive/Shortcuts/LifeTrace/photo-sync-config.json
```

推荐“在我的 iPhone 上”，减少令牌进入云端同步的范围。配置格式：

```json
{
  "server": "https://192.168.3.30:3443",
  "stableServer": "https://lifetrace.local:3443",
  "deviceId": "持久化随机设备标识",
  "deviceName": "我的 iPhone",
  "deviceToken": "配对后返回的长期 Token",
  "lastSuccessfulCaptureTime": null
}
```

`deviceToken` 相当于该 iPhone 的上传密码。不要截图、分享配置文件或把它写入另一个二维码。LifeTrace 数据库只保存令牌哈希，电脑端无法再次查看明文令牌。

LifeTrace 会为每台电脑首次生成独立的本地 CA，CA 私钥通过系统安全存储保护，不再随安装包复用。更换 Wi‑Fi 或局域网 IP 后，应用会自动更新服务器证书，iPhone 不需要重新安装 CA。只有清空 LifeTrace 应用数据、重装电脑系统、换电脑或在 iPhone 删除证书时才需要重新安装。

二维码会把当前电脑 IP 写入 `server`，保证配对和上传请求不依赖路由器的 mDNS 支持；同时把 `https://lifetrace.local:3443` 写入可选的 `stableServer`。如果路由器支持 mDNS，后续可以在快捷指令中先尝试稳定地址，再回退到当前 IP。电脑 IP 变化且该网络不支持 mDNS 时，重新扫描二维码即可更新地址，但不需要重新安装根证书。

### 快捷指令仍提示“服务器证书无效”

如果证书已安装并开启“完全信任”，但“获取 URL 内容”仍然拒绝请求，可以在
LifeTrace 的“照片”模块找到“局域网服务”，点击“快捷指令证书无效？开启 HTTP
兼容模式”。阅读风险提示并确认后，重新点击“添加 iPhone”生成二维码。

兼容模式下二维码地址以 `http://` 开头，不再需要 iPhone 验证本地证书。设备 Token、
一次性配对码、局域网来源限制、上传大小限制和媒体文件校验仍然生效；但 HTTP 不提供
传输加密，同一网络中的恶意设备可能监听令牌或照片内容。因此该模式默认关闭，只应在
可信的家庭局域网中临时使用。完成排障后，可点击“关闭兼容模式，恢复 HTTPS”。切换
模式后必须重新生成二维码，旧二维码中的协议和地址不会自动改变。

## 快捷指令总体结构

快捷指令有 `pair` 和 `sync` 两种模式：

```text
取得“快捷指令输入”
→ 如果输入有值，尝试把输入作为 JSON 解析
→ 如果 action 等于 pair，运行配对模式并结束
→ 否则运行同步模式
```

二维码传入的内容类似：

```json
{
  "action": "pair",
  "server": "https://192.168.3.30:3443",
  "stableServer": "https://lifetrace.local:3443",
  "pairCode": "482913",
  "expiresAt": "2026-07-26T23:30:00+08:00"
}
```

二维码只携带短期配对码，不携带长期令牌。配对码默认 5 分钟失效，只能成功使用一次。

## 配对模式

在 `action = pair` 分支中依次添加以下动作：

1. 使用“从输入中获取字典”解析快捷指令输入。
2. 读取 `server`、`pairCode` 和 `expiresAt`。
3. 检查配置文件是否存在。
4. 如果配置已存在，读取原有 `deviceId`；否则创建一个至少 20 位的随机设备标识。新版 iOS 没有“生成 UUID”时，可将格式化日期 `yyyyMMddHHmmss` 与两个 9 位随机数连续拼接。这个值以后必须持续复用。
5. 使用“获取设备详细信息”取得设备名称，或第一次运行时询问用户输入名称。
6. 添加“获取 URL 内容”：
   - URL：`server` + `/api/photo-sync/pair`
   - 方法：`POST`
   - 请求体：JSON
   - `pairCode`：二维码传入值
   - `deviceName`：设备名称
   - `deviceId`：持久化 UUID
7. 检查 HTTP 结果中的 `success`。只有 `success = true` 才继续。
8. 从响应读取 `deviceToken` 和服务端返回的 `deviceId`。
9. 生成完整配置字典，`lastSuccessfulCaptureTime` 初始设为“空值”。
10. 用“存储文件”覆盖保存 `photo-sync-config.json`。
11. 显示“LifeTrace 照片同步配对成功”并结束快捷指令。

配对失败时不要创建或覆盖有效配置。常见错误：

| 错误码 | 处理 |
| --- | --- |
| `PAIR_CODE_INVALID` | 回到 LifeTrace 重新生成二维码 |
| `PAIR_CODE_EXPIRED` | 配对码已过期，重新生成 |
| `PAIR_CODE_ALREADY_USED` | 此码已完成配对，重新生成 |
| HTTP 429 | 等待一分钟后重新配对 |

## 同步模式

### 1. 读取配置和健康检查

1. 读取 `photo-sync-config.json`，不存在则提示先在电脑端添加 iPhone。
2. 从配置读取 `server` 和 `deviceToken`。
3. 调用：

```http
GET /api/photo-sync/health
Authorization: Bearer <deviceToken>
```

在“获取 URL 内容”动作中，把请求头 `Authorization` 设为 `Bearer ` 加令牌。健康检查失败、超时或电脑离线时立即结束，不修改 `lastSuccessfulCaptureTime`。

### 2. 查找本次候选照片

1. 如果 `lastSuccessfulCaptureTime` 有值，用“调整日期”减去 10 分钟，得到查询起始时间。
2. 如果是第一次运行，询问用户选择开始日期；推荐只选择当天或最近几天，不要默认同步整个相册。
3. 使用“查找照片”：
   - 创建日期晚于查询起始时间；
   - 按创建日期升序；
   - 限制 10～20 项，推荐 15 项。
4. 逐项重复。对每项取得：
   - 原始文件；
   - 文件名；
   - 媒体类型；
   - MIME 类型；
   - 拍摄日期；
   - 文件大小。
5. 为 `clientAssetId` 生成可重复的标识。快捷指令没有稳定资源 ID 时，可以把“设备 UUID + 拍摄时间 + 原始文件名 + 文件大小”组合后计算 SHA‑256。它只是快速预检查标识，服务端最终仍会对原文件内容计算 SHA‑256。

### 3. 创建上传任务

调用：

```http
POST /api/photo-sync/assets
Authorization: Bearer <deviceToken>
Content-Type: application/json
```

JSON 请求体：

```json
{
  "clientAssetId": "稳定资源标识",
  "fileName": "IMG_4821.HEIC",
  "mediaType": "image",
  "mimeType": "image/heic",
  "capturedAt": "2026-07-26T18:31:25+08:00",
  "fileSize": 4283182
}
```

如果响应 `alreadyExists = true`，把该项视为成功，更新本地成功进度并继续下一项。

### 4. 上传原始文件

如果响应包含 `uploadId`，添加另一个“获取 URL 内容”动作：

```http
PUT /api/photo-sync/assets/{uploadId}/content
Authorization: Bearer <deviceToken>
Content-Type: application/octet-stream
```

方法选择 `PUT`，请求体类型选择“文件”，值选择当前照片的原始文件变量。不要把文件转换为文本或 Base64，不要使用 multipart 表单。

### 5. 完成上传

文件上传成功后调用：

```http
POST /api/photo-sync/assets/{uploadId}/complete
Authorization: Bearer <deviceToken>
```

只有以下任一结果才能把当前项视为成功：

- `success = true` 且 `duplicate = false`；
- `success = true` 且 `duplicate = true`；
- 创建任务时返回 `alreadyExists = true`。

每成功一项，就把该照片的拍摄时间写入配置的 `lastSuccessfulCaptureTime`，然后立即覆盖保存配置文件。任意一项失败时：

1. 记录本次失败信息；
2. 停止处理后续照片；
3. 不推进失败照片对应的时间；
4. 显示失败摘要；
5. 下次运行从重叠时间窗重新尝试。

这样即使快捷指令、网络或电脑在中途退出，也不会跳过未确认成功的照片。10 分钟重叠可能再次提交已成功项目，服务端会通过设备资源映射快速返回，最终仍由 SHA‑256 内容哈希防止重复保存。

## 媒体类型映射

| iPhone 文件 | `mediaType` | 建议 MIME |
| --- | --- | --- |
| HEIC / HEIF | `image` | `image/heic` / `image/heif` |
| JPEG | `image` | `image/jpeg` |
| PNG | `image` | `image/png` |
| MOV | `video` | `video/quicktime` |
| MP4 | `video` | `video/mp4` |

Live Photo 第一版会把静态照片和对应 MOV 作为两个独立资源同步。

## 同步错误处理

| 错误码 | 快捷指令行为 |
| --- | --- |
| `DEVICE_TOKEN_INVALID` | 停止同步，回到电脑端重新配对 |
| `DEVICE_REVOKED` | 停止同步，重新配对前不要继续 |
| `UPLOAD_FILE_TOO_LARGE` | 停止并提示调整电脑端限制 |
| `UNSUPPORTED_MEDIA_TYPE` / `INVALID_MEDIA_FILE` | 停止，保留进度在上一项 |
| `UPLOAD_SIZE_MISMATCH` | 停止，下次重新创建/上传任务 |
| `SERVER_STORAGE_ERROR` | 停止，不推进同步时间 |
| `PHOTO_PROCESSING_FAILED` | 原文件通常已保存；在电脑端任务列表重试媒体处理 |

## 创建个人自动化

在“快捷指令 → 自动化”中创建个人自动化，选择以下任一触发条件：

- 每天晚上固定时间；
- 接入充电器；
- 连接指定的家庭 Wi‑Fi。

自动化执行 `LifeTrace照片同步`，不要传入配对 JSON，此时快捷指令进入 `sync` 模式。是否能“立即运行”取决于当前 iOS 版本、所选触发器和用户的自动化设置。

快捷指令不是常驻后台服务。LifeTrace 提供的是定期增量同步，不承诺像原生照片应用一样实时或持续后台上传。电脑离线时健康检查会失败，本次运行立即停止并保留原进度；电脑下次在线后再次运行即可补传。
