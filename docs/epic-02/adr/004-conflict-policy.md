# ADR-004：显式冲突，禁止默认 Last-Write-Wins

- 状态：已采纳（EPIC-02）

## 背景

多设备离线编辑同一实体时，若默认 LWW，低价值后写会覆盖高价值先写，且用户无感知。

## 决策

- 当 `client baseServerVersion != current serverVersion`（或实体状态冲突）时，服务端返回 `conflict` 并附当前实体/tombstone。
- v1 不自动解决冲突；解决方式由客户端实现：`keep_server` / `keep_local` / `manual_merge`。
- `keep_local` 必须生成新 changeId 并基于最新服务端版本重新提交。

## 理由

- 个人数据场景中无声覆盖不可接受。
- 显式冲突给用户控制权，且实现简单可靠。

## 后果

- 客户端需要冲突处理路径与 UI（属于 EPIC-05，不在本 Epic）。
- `user_owned` 实体一律 `conflict_mode=optimistic`；`server_managed`/`shared_catalog` 使用 `server_authoritative`。
