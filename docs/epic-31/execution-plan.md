# EPIC-31 相册设备直连同步执行计划

> 状态：待执行  
> 日期：2026-08-10  
> 目标：在不把照片持久化到云端的前提下，实现 Android → Windows 的可靠直连同步，并提供 P2P 失败时的 TURN 流式兜底。

## 1. 第一阶段范围

### 实现

- 设备注册与设备身份；
- Android / Windows 设备配对；
- 云端在线状态；
- WebRTC 信令；
- STUN；
- TURN 兜底；
- LAN 本地发现；
- DataChannel 文件传输；
- manifest / chunk 协议；
- 断点续传；
- SHA-256 校验；
- 原子落盘；
- pending / retry 队列；
- 链路类型与速度展示；
- 私密相册隔离；
- 安全与故障测试。

### 不实现

- 云端照片存储；
- 云端离线缓存；
- 云端缩略图；
- 云端照片备份；
- EPIC-30 私密媒体同步；
- 依赖 TURN 作为默认大文件传输路径。

## 2. 阶段 A：设备身份与在线状态

- [ ] 为 Android / Windows 生成稳定 `device_id`
- [ ] 生成设备密钥并安全存储
- [ ] 云端设备注册
- [ ] 用户只可连接自己账号授权的设备
- [ ] WebSocket / presence 在线状态
- [ ] 设备最后在线时间
- [ ] 设备撤销
- [ ] challenge-response 身份校验

## 3. 阶段 B：本地发现

- [ ] Windows 发布 mDNS / DNS-SD 服务
- [ ] Android NSD / mDNS 发现
- [ ] 只广播最小设备元数据
- [ ] 发现后验证设备身份
- [ ] 云端不可用时仍可通过 LAN 找到已配对 Windows
- [ ] AP isolation / 不可达时安全失败并进入其他路径

## 4. 阶段 C：WebRTC 信令与 ICE

- [ ] Cloud Signaling endpoint / WebSocket
- [ ] offer / answer 交换
- [ ] ICE candidate 交换
- [ ] STUN 配置
- [ ] selected candidate pair 诊断
- [ ] `lan / p2p / relay` 链路分类
- [ ] 连接超时与取消
- [ ] 信令会话自动过期

验收：同一 LAN 可稳定选中本地直连路径；不同网络在 NAT 允许时可选中公网 P2P。

## 5. 阶段 D：TURN 兜底

- [ ] 部署 TURN 服务
- [ ] 使用短时凭据
- [ ] 限制匿名使用
- [ ] 限制单连接 / 单用户资源
- [ ] TURN 不落盘照片
- [ ] TURN 日志不包含内容
- [ ] P2P 失败后自动退到 relay
- [ ] UI 明确显示“云端中转”

## 6. 阶段 E：传输协议

- [ ] `TransferManifest`
- [ ] `Chunk`
- [ ] `ChunkAck`
- [ ] `ResumeRequest`
- [ ] `FinalAck`
- [ ] `TransferError`
- [ ] 协议版本字段
- [ ] 最大 chunk 大小
- [ ] backpressure
- [ ] 取消传输

## 7. 阶段 F：Android 队列

- [ ] 扫描 / 选择待同步照片
- [ ] 生成 `assetId`
- [ ] 计算内容 hash
- [ ] `pending / connecting / transferring / verifying / completed / failed`
- [ ] PC 离线保持 pending
- [ ] 记录 confirmed offset
- [ ] 网络中断后重连续传
- [ ] final ACK 后才 completed
- [ ] 用户可重试 failed 任务

## 8. 阶段 G：Windows 接收端

- [ ] 接收 manifest
- [ ] 判断是否已存在同 hash 对象
- [ ] 创建 `.part`
- [ ] 按 offset 写入
- [ ] checkpoint
- [ ] 完整 size 校验
- [ ] SHA-256 校验
- [ ] 原子 rename
- [ ] 写入照片 Repository / SQLite 索引
- [ ] 返回 final ACK
- [ ] 异常退出后恢复 / 清理孤立 `.part`

## 9. 阶段 H：私密媒体硬隔离

- [ ] 同步调度器拒绝 `local-private`
- [ ] manifest 构建器拒绝私密媒体
- [ ] DataChannel 发送器拒绝私密媒体
- [ ] TURN 路径拒绝私密媒体
- [ ] 日志 / 诊断不记录私密元数据
- [ ] 增加 CI 回归测试防止 EPIC-30 被接入远程同步

## 10. 阶段 I：UI 与可观测性

Android / Windows 至少展示：

```text
目标设备
在线 / 离线
当前链路：局域网直连 / P2P / 云端中转
当前速度
当前文件进度
总进度
重试状态
```

- [ ] 连接阶段状态
- [ ] 每文件进度
- [ ] 总任务进度
- [ ] 当前吞吐
- [ ] 失败原因
- [ ] 重试入口
- [ ] PC 离线提示

## 11. 测试矩阵

### LAN

- [ ] Android Wi-Fi + Windows 有线，同一路由器
- [ ] Android / Windows 都为 Wi-Fi
- [ ] 云端信令不可用但本地发现可用
- [ ] AP isolation 导致 LAN 直连失败

### Internet P2P

- [ ] 不同家庭网络
- [ ] 手机 5G + 家庭宽带
- [ ] NAT 穿透成功
- [ ] NAT 穿透失败后 TURN fallback

### 传输可靠性

- [ ] 1 张小图
- [ ] 大图
- [ ] 视频
- [ ] 数百文件批量
- [ ] 传输中断网
- [ ] Android 切换 Wi-Fi / 5G
- [ ] Windows 重启
- [ ] hash mismatch
- [ ] 重复发送
- [ ] 磁盘空间不足

### 安全

- [ ] 未授权设备拒绝
- [ ] 被撤销设备拒绝
- [ ] 伪造信令拒绝
- [ ] TURN 匿名滥用拒绝
- [ ] 私密相册媒体无法进入任意网络路径

## 12. 完成定义

- [ ] LAN 同步时云服务器照片流量为 0
- [ ] Internet P2P 成功时 TURN 照片流量为 0
- [ ] 只有直连失败才使用 TURN
- [ ] TURN 中继过程中服务器不创建照片文件
- [ ] PC 离线时云端没有照片副本
- [ ] 断点续传可恢复到已确认 offset
- [ ] Windows hash 校验失败时不会产生正式文件
- [ ] final ACK 前 Android 不标记 completed
- [ ] 云端故障时已配对设备仍可在 LAN 内同步
- [ ] EPIC-30 私密媒体测试全部通过
