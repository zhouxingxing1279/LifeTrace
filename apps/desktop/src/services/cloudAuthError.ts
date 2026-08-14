type ErrorShape = {
  message?: unknown;
  status?: unknown;
  code?: unknown;
};

function errorShape(error: unknown): ErrorShape | undefined {
  if (!error || typeof error !== "object") return undefined;
  return error as ErrorShape;
}

export function rawCloudAuthErrorMessage(error: unknown): string {
  if (typeof error === "string") return error.trim();
  if (error instanceof Error) return error.message.trim();
  const shape = errorShape(error);
  return typeof shape?.message === "string" ? shape.message.trim() : "";
}

function numericStatus(error: unknown): number | undefined {
  const value = errorShape(error)?.status;
  return typeof value === "number" && Number.isFinite(value) ? value : undefined;
}

function errorCode(error: unknown): string {
  const value = errorShape(error)?.code;
  return typeof value === "string" ? value.toLowerCase() : "";
}

export function cloudAuthErrorMessage(error: unknown, fallback: string): string {
  const status = numericStatus(error);
  const code = errorCode(error);
  const raw = rawCloudAuthErrorMessage(error);
  const normalized = raw.toLowerCase();

  if (status === 401 || code.includes("auth_invalid")) return "邮箱或密码错误";
  if (status === 423 || code.includes("auth_user_locked")) return "账号暂时锁定，请稍后再试";
  if (status === 429 || code.includes("auth_rate_limited")) return "登录尝试过多，请稍后再试";
  if (code.includes("auth_user_disabled")) return "账号已停用";
  if (code.includes("auth_device_revoked")) return "当前设备的登录授权已被撤销";
  if (code.includes("auth_app_revoked")) return "当前应用的账号授权已被撤销";
  if (code.includes("auth_scope_denied")) return "当前应用没有所需的云端权限";
  if (status === 502 || status === 503 || status === 504) return "云端认证服务暂时不可用，请稍后重试";
  if (status === 400 && (code.includes("app_id_unsupported") || normalized.includes("unsupported application"))) {
    return "当前客户端版本与云端不兼容，请更新 LifeTrace";
  }
  if (
    normalized.includes("failed to fetch") ||
    normalized.includes("network request") ||
    normalized.includes("无法连接 lifetrace 云端") ||
    normalized.includes("error sending request") ||
    normalized.includes("connection refused") ||
    normalized.includes("dns error")
  ) {
    return "无法连接 LifeTrace 云端，请检查服务器地址和网络连接";
  }
  if (normalized.includes("windows credential manager")) {
    return `无法保存 Windows 安全登录凭据：${raw}`;
  }

  return raw || fallback;
}
