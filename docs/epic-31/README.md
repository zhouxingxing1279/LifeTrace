# EPIC-31：相册设备直连同步与云端中继控制面

> 状态：规划  
> 日期：2026-08-10

## 目标

实现 Android 手机与 Windows LifeTrace 照片库之间的直接同步：照片本体不持久化到 LifeTrace Cloud；同一局域网优先局域网直传，不同网络优先公网 P2P，只有直连失败时才通过 TURN 流式中转。

```text
优先级：

LAN direct
   ↓ 不可达
Internet P2P
   ↓ 不可达
TURN relay
```

如果目标 PC 离线，手机保留待同步任务，等待 PC 再次上线；云端不临时缓存照片。

## 核心边界

### 云端只做控制面

LifeTrace Cloud 可以负责：

- 用户鉴权；
- 设备注册与设备身份；
- 在线状态；
- WebRTC 信令；
- STUN；
- TURN 兜底中继。

LifeTrace Cloud 不负责：

- 照片原图持久化；
- 视频持久化；
- 离线照片缓存；
- 云端缩略图；
- 照片备份仓库；
- 私密相册同步。

### 私密相册永久排除

EPIC-30 的私密媒体、私密缩略图、私密元数据和密钥全部保持 `local-only`，EPIC-31 必须显式拒绝它们进入普通照片同步链路。

## 传输模型

```text
Android
   │
   │ 1. 发现 / 选择 Windows 设备
   ▼
Device Discovery
   │
   │ 2. 建立 WebRTC / ICE
   ▼
┌───────────────┬────────────────┬──────────────┐
│ LAN host pair │ Internet P2P   │ TURN relay   │
└───────┬───────┴───────┬────────┴──────┬───────┘
        └─────────────── DataChannel ────┘
                         │
                         ▼
                    Chunk Transfer
                         │
                         ▼
                 Windows temporary file
                         │
                         ▼
                    SHA-256 verify
                         │
                         ▼
                     atomic rename
                         │
                         ▼
                        ACK
                         │
                         ▼
                  Android completed
```

## 离线策略

PC 不在线时：

```text
照片待同步
→ Android 本地 pending
→ 不上传云端
→ PC 上线
→ 自动 / 手动重新建立传输
→ 从已确认 offset 继续
```

手机只有收到 Windows 端“文件落盘且哈希校验成功”的 ACK 后，才能把任务标记为 `completed`。

## 状态机

```text
pending
connecting
transferring
verifying
completed
failed
```

`failed` 必须保留可重试信息；网络中断不能把已成功确认的 chunk 重新全部发送。

## 验收标准

- [ ] 同一局域网内照片字节不经过云服务器
- [ ] 不同网络在 NAT 穿透成功时照片字节不经过 TURN
- [ ] P2P 失败时可以通过 TURN 流式中继且服务器不落盘照片
- [ ] PC 离线时照片只留在 Android 待同步队列
- [ ] 中断后可断点续传
- [ ] Windows 端完成 SHA-256 校验后才返回最终 ACK
- [ ] 重复照片不会因为重复任务产生多份副本
- [ ] 已配对设备在同一 LAN 时可以通过本地发现建立直连
- [ ] 云端信令故障时，已配对设备在同一 LAN 仍有本地发现 / 直连路径
- [ ] EPIC-30 私密媒体无法进入该同步链路

详细设计见：

- `architecture.md`
- `execution-plan.md`
