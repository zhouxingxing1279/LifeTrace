import type { ShoppingPlatform, ShoppingSyncCursor, VerificationRequirement } from "./types";
import type { ShoppingError } from "./errors";

export type ShoppingSyncStatus =
  | "idle"
  | "checking_connection"
  | "syncing"
  | "verification_required"
  | "paused"
  | "completed"
  | "failed";

export type ShoppingSyncState =
  | { status: "idle"; platform: ShoppingPlatform }
  | { status: "checking_connection"; platform: ShoppingPlatform }
  | { status: "syncing"; platform: ShoppingPlatform; page: number; collected: number }
  | {
      status: "verification_required";
      platform: ShoppingPlatform;
      verification: VerificationRequirement;
      collected: number;
    }
  | { status: "paused"; platform: ShoppingPlatform; reason: "auth_required" | "cancelled"; collected: number }
  | {
      status: "completed";
      platform: ShoppingPlatform;
      pages: number;
      collected: number;
      cursor?: ShoppingSyncCursor;
    }
  | { status: "failed"; platform: ShoppingPlatform; error: ShoppingError; collected: number };
