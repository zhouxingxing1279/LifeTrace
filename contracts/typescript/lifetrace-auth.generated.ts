// GENERATED FILE - DO NOT EDIT MANUALLY
// LifeTrace authentication protocol v1.
// Rust types in crates/lifetrace-contracts are authoritative.

import type { AppId, UserId } from "./lifetrace-contracts.generated";

export type AcceptedResponseV1 = { accepted: boolean, };

export type AppGrantId = string;

export type AppGrantListV1 = { grants: Array<AppGrantV1>, };

export type AppGrantV1 = { id: AppGrantId, appId: AppId, scopes: Array<Scope>, status: string, grantedAt: string, updatedAt: string, revokedAt: string | null, };

export type AppInstallationId = string;

export type AuthCapabilitiesV1 = { registrationMode: string, passwordMinLength: number, passwordMaxBytes: number, accessTokenTtlSeconds: bigint, refreshIdleTtlSeconds: bigint, refreshAbsoluteTtlSeconds: bigint, webSessionEnabled: boolean, supportedApps: Array<AppId>, };

export type AuthSessionId = string;

export type AuthSessionV1 = { id: AuthSessionId, appId: AppId, deviceId: AppInstallationId, sessionType: string, status: string, scopes: Array<Scope>, publicDevice: boolean, createdAt: string, lastSeenAt: string, idleExpiresAt: string, absoluteExpiresAt: string, revokedAt: string | null, current: boolean, };

export type AuthUserV1 = { id: UserId, email: string, displayName: string | null, state: string, emailVerifiedAt: string | null, createdAt: string, passwordChangedAt: string | null, };

export type ChangePasswordRequestV1 = { currentPassword: string, newPassword: string, };

export type CsrfResponseV1 = { csrfToken: string, };

export type DeviceInstallationV1 = { id: AppInstallationId, externalDeviceId: string, deviceGroupId: string | null, deviceName: string, appId: AppId, platform: string, status: string, clientVersion: string | null, firstSeenAt: string, lastSeenAt: string, lastLoginAt: string | null, lastSyncAt: string | null, revokedAt: string | null, current: boolean, };

export type DeviceListV1 = { devices: Array<DeviceInstallationV1>, };

export type ForgotPasswordRequestV1 = { email: string, };

export type LoginRequestV1 = { email: string, password: string, appId: AppId, deviceId: string, deviceName: string, platform: string, clientVersion: string | null, requestedScopes: Array<Scope>, publicDevice: boolean, };

export type RefreshRequestV1 = { refreshToken: string, appId: AppId, deviceId: string, };

export type RegisterRequestV1 = { email: string, password: string, displayName: string | null, inviteToken: string | null, appId: AppId, deviceId: string, deviceName: string, platform: string, clientVersion: string | null, requestedScopes: Array<Scope>, };

export type ResetPasswordRequestV1 = { token: string, newPassword: string, };

export type Scope = string;

export type SessionListV1 = { sessions: Array<AuthSessionV1>, };

export type TokenFamilyId = string;

export type TokenResponseV1 = { accessToken: string, refreshToken: string | null, tokenType: string, expiresIn: bigint, refreshExpiresIn: bigint | null, user: AuthUserV1, session: AuthSessionV1, scopes: Array<Scope>, };

export type UpdateAppGrantRequestV1 = { scopes: Array<Scope>, };

export type UpdateDeviceRequestV1 = { deviceName: string, };

export type WebLoginRequestV1 = { email: string, password: string, requestedScopes: Array<Scope>, publicDevice: boolean, };

export type WebSessionResponseV1 = { user: AuthUserV1, session: AuthSessionV1, csrfToken: string, };

