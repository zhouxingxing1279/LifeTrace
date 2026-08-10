import type { ShoppingPlatform, VerificationRequirement } from "./types";

export type ShoppingErrorCode =
  | "AUTH_REQUIRED"
  | "VERIFICATION_REQUIRED"
  | "RATE_LIMITED"
  | "ACCESS_DENIED"
  | "SOURCE_UNAVAILABLE"
  | "PARSE_FAILED"
  | "NORMALIZE_FAILED"
  | "CANCELLED"
  | "UNKNOWN";

export interface ShoppingErrorOptions {
  platform?: ShoppingPlatform;
  retryable?: boolean;
  verification?: VerificationRequirement;
  cause?: unknown;
}

export class ShoppingError extends Error {
  readonly code: ShoppingErrorCode;
  readonly platform?: ShoppingPlatform;
  readonly retryable: boolean;
  readonly verification?: VerificationRequirement;

  constructor(code: ShoppingErrorCode, message: string, options: ShoppingErrorOptions = {}) {
    super(message, { cause: options.cause });
    this.name = "ShoppingError";
    this.code = code;
    this.platform = options.platform;
    this.retryable = options.retryable ?? false;
    this.verification = options.verification;
  }
}

export function isShoppingError(error: unknown): error is ShoppingError {
  return error instanceof ShoppingError;
}

export function asShoppingError(error: unknown, platform?: ShoppingPlatform): ShoppingError {
  if (isShoppingError(error)) return error;
  if (error instanceof Error) {
    return new ShoppingError("UNKNOWN", error.message, { platform, cause: error });
  }
  return new ShoppingError("UNKNOWN", "Unknown shopping collector failure", { platform, cause: error });
}
