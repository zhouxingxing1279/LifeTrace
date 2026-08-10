# EPIC-31 相册设备直连同步架构

> 日期：2026-08-10  
> 状态：规划

## 1. 架构原则

照片同步采用“云端控制面 + P2P 数据面”。云服务器负责帮助设备找到彼此并建立连接，但不是照片存储中心。

```text
                       LifeTrace Cloud
                 ┌────────────────────────┐
                 │ Auth / Device Registry │
                 │ Presence / Signaling   │
                 │ STUN / TURN            │
                 └───────────┬────────────┘
                             │
                  只交换控制与连接信息
                             │
          ┌──────────────────┴──────────────────┐
          ▼                                     ▼
   Android Photo Sync                    Windows LifeTrace
          │                                     │
          └────────── WebRTC DataChannel ───────┘
```

## 2. 连接路径选择

使用 WebRTC ICE 自动收集和测试候选路径：

- `host`：局域网可达地址；
- `srflx`：通过 STUN 得到的 NAT 映射地址；
- `relay`：TURN 中继地址。

候选优先级由 ICE 连通性检查决定，业务层只需要读取最终 selected candidate pair 并展示链路类型。

UI 可以显示：

```text
局域网直连
公网 P2P
云端中转
```

## 3. 本地发现

为了让同一局域网在云端暂时不可用时仍可同步，已配对设备增加本地发现能力。

建议：

- Windows 发布 mDNS / DNS-SD 服务；
- Android 使用 NSD / mDNS 发现；
- 服务记录只暴露设备 ID、协议版本和临时连接端口，不暴露照片文件名；
- 发现后仍执行设备身份握手，不把“在同一 LAN”视为可信身份。

本地发现用于“找到设备”，最终传输仍复用统一的数据通道和传输协议。

## 4. 设备身份与配对

每台设备注册独立 `device_id` 和设备密钥。首次配对由同一 LifeTrace 用户身份授权，之后每次连接进行 challenge-response。

目标：

- 同一局域网里的陌生设备不能冒充 Windows 客户端；
- TURN 中继不能伪造目标设备；
- 信令服务器只能安排连接，不能代替设备身份认证。

设备密钥不得进入普通日志和导出包。

## 5. 云端信令

信令只携带建立连接所需的短期数据，例如：

- source device id；
- target device id；
- offer / answer；
- ICE candidate；
- session nonce；
- 短时会话状态。

照片文件名、相册路径、照片内容和长期同步队列不通过信令服务保存。

信令会话在连接结束后快速过期。

## 6. TURN 边界

TURN 只作为最后兜底：

```text
Android → TURN → Windows
```

要求：

- 不写照片文件到磁盘；
- 不把媒体内容进入应用日志；
- 配置带宽 / 会话限额；
- 只允许已认证设备获取短时 TURN 凭据；
- 能观测中继流量，但不记录照片业务内容。

2C2G / 3Mbps 云服务器可以承担低频兜底，但大批量照片同步应尽量依赖 LAN / P2P。

## 7. 传输协议

每个待传对象先发送 manifest：

```text
assetId
contentHash
size
mimeType
capturedAt
relativeTarget?
chunkSize
```

不把本地绝对路径作为跨端协议字段。

### Chunk

```text
transferId
offset
length
payload
```

Windows 每确认一段连续 offset，Android 更新本地 checkpoint。

## 8. Windows 落盘协议

Windows 必须采用临时文件 + 校验 + 原子提交：

```text
receive chunks
→ <asset>.part
→ 完整 size 检查
→ SHA-256
→ hash match
→ atomic rename
→ 写照片索引 / SQLite 元数据
→ final ACK
```

只有 final ACK 才代表 Android 可以把任务标记为完成。

如果 hash 不匹配：

- 不覆盖现有正式文件；
- 删除或隔离 `.part`；
- 返回 `VERIFY_FAILED`；
- Android 保留任务并允许重试。

## 9. 断点续传

双方维护轻量 transfer checkpoint：

```text
transfer_id
asset_id
content_hash
confirmed_offset
updated_at
```

重新连接时先比较 `content_hash` 和已确认 offset：

- 相同文件：从 confirmed offset 继续；
- 文件已完整存在且 hash 相同：直接 ACK / dedupe；
- hash 不同：创建新 transfer，不覆盖错误对象。

## 10. 去重

优先使用：

```text
contentHash + size
```

`assetId` 用于追踪移动端照片资产身份，hash 用于确认内容是否一致。

在 Windows 端发现相同内容时，可以复用现有文件记录或只建立新的逻辑引用，具体相册业务规则由照片 Repository 决定。

## 11. PC 离线

云端不缓存照片。

```text
Android pending queue
        │
        ├── PC offline → 保留
        │
        └── PC online → connecting → transfer
```

待同步队列与照片内容都留在手机。云端在线状态只用于快速判断目标是否可连接。

## 12. 私密相册隔离

任何带有 EPIC-30 `local-private` 标记的媒体必须在进入同步调度器之前被拒绝。

需要自动化测试确保：

- 不生成 transfer manifest；
- 不进入 WebRTC DataChannel；
- 不进入 TURN；
- 不进入同步日志；
- 不进入本地发现元数据。

## 13. 可观测性

允许记录：

- transfer id；
- device id；
- 链路类型；
- 字节数；
- 速度；
- 时长；
- 重试次数；
- 错误码。

禁止记录：

- 私密照片信息；
- 文件内容；
- 设备密钥；
- TURN 长期凭据；
- 绝对文件路径；
- 未脱敏的照片文件名（诊断默认关闭）。
