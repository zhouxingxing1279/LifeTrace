import type { ShoppingAdapter } from "./adapter";
import { asShoppingError, ShoppingError } from "./errors";
import type { ShoppingSyncState } from "./sync-state";
import {
  getUnifiedOrderKey,
  validateUnifiedOrder,
  type ShoppingSyncCursor,
  type UnifiedOrder,
  type VerificationRequirement,
} from "./types";

export interface ShoppingSyncOptions {
  cursor?: ShoppingSyncCursor;
  /** Existing order keys used as the newest-order traversal boundary. */
  knownOrderKeys?: Iterable<string>;
  /** Known active orders that should still be refreshed after reaching the list boundary. */
  refreshOrderIds?: Iterable<string>;
  signal?: AbortSignal;
  /** Hard safety guard, not a normal pagination policy. */
  maxPages?: number;
  onBatch?: (orders: readonly UnifiedOrder[]) => Promise<void> | void;
  onStateChange?: (state: ShoppingSyncState) => Promise<void> | void;
}

const DEFAULT_MAX_PAGES = 100;

async function publish(options: ShoppingSyncOptions, state: ShoppingSyncState): Promise<ShoppingSyncState> {
  await options.onStateChange?.(state);
  return state;
}

function fallbackVerification(message?: string): VerificationRequirement {
  return { type: "unknown", message };
}

async function cancelled(
  adapter: ShoppingAdapter<unknown>,
  options: ShoppingSyncOptions,
  collected: number,
): Promise<ShoppingSyncState> {
  return publish(options, { status: "paused", platform: adapter.platform, reason: "cancelled", collected });
}

async function failureFromError(
  adapter: ShoppingAdapter<unknown>,
  options: ShoppingSyncOptions,
  error: unknown,
  collected: number,
): Promise<ShoppingSyncState> {
  if (options.signal?.aborted) return cancelled(adapter, options, collected);

  const shoppingError = asShoppingError(error, adapter.platform);
  if (shoppingError.code === "VERIFICATION_REQUIRED") {
    return publish(options, {
      status: "verification_required",
      platform: adapter.platform,
      verification: shoppingError.verification ?? fallbackVerification(shoppingError.message),
      collected,
    });
  }
  if (shoppingError.code === "AUTH_REQUIRED") {
    return publish(options, { status: "paused", platform: adapter.platform, reason: "auth_required", collected });
  }
  if (shoppingError.code === "CANCELLED") return cancelled(adapter, options, collected);

  return publish(options, { status: "failed", platform: adapter.platform, error: shoppingError, collected });
}

function validateAdapterOrder(adapter: ShoppingAdapter<unknown>, order: UnifiedOrder): void {
  if (order.platform !== adapter.platform) {
    throw new TypeError(`adapter platform ${adapter.platform} normalized order for ${order.platform}`);
  }
  validateUnifiedOrder(order);
}

async function emitBatch(
  adapter: ShoppingAdapter<unknown>,
  options: ShoppingSyncOptions,
  batch: readonly UnifiedOrder[],
  collected: number,
): Promise<{ collected: number; failure?: ShoppingSyncState }> {
  if (batch.length === 0) return { collected };
  try {
    await options.onBatch?.(batch);
    return { collected: collected + batch.length };
  } catch (error) {
    return { collected, failure: await failureFromError(adapter, options, error, collected) };
  }
}

export async function runShoppingSync<TRawOrder>(
  adapter: ShoppingAdapter<TRawOrder>,
  options: ShoppingSyncOptions = {},
): Promise<ShoppingSyncState> {
  const genericAdapter = adapter as ShoppingAdapter<unknown>;
  const knownOrderKeys = new Set(options.knownOrderKeys ?? []);
  const seenOrderKeys = new Set<string>();
  const maxPages = options.maxPages ?? DEFAULT_MAX_PAGES;
  let collected = 0;
  let pages = 0;
  let pageToken: string | undefined;
  let checkpoint = options.cursor;
  let checkpointCaptured = false;

  if (!Number.isSafeInteger(maxPages) || maxPages <= 0) {
    throw new TypeError("maxPages must be a positive safe integer");
  }

  await publish(options, { status: "idle", platform: adapter.platform });
  if (options.signal?.aborted) return cancelled(genericAdapter, options, collected);

  await publish(options, { status: "checking_connection", platform: adapter.platform });

  let connection;
  try {
    connection = await adapter.checkConnection(options.signal);
  } catch (error) {
    return failureFromError(genericAdapter, options, error, collected);
  }

  if (options.signal?.aborted) return cancelled(genericAdapter, options, collected);

  if (connection.status === "auth_required") {
    return publish(options, { status: "paused", platform: adapter.platform, reason: "auth_required", collected });
  }
  if (connection.status === "verification_required") {
    return publish(options, {
      status: "verification_required",
      platform: adapter.platform,
      verification: connection.verification ?? fallbackVerification(connection.message),
      collected,
    });
  }
  if (connection.status === "unavailable") {
    return publish(options, {
      status: "failed",
      platform: adapter.platform,
      error: new ShoppingError("SOURCE_UNAVAILABLE", connection.message ?? "Shopping source is unavailable", {
        platform: adapter.platform,
        retryable: true,
      }),
      collected,
    });
  }

  let listTraversalComplete = false;
  while (!listTraversalComplete) {
    if (options.signal?.aborted) return cancelled(genericAdapter, options, collected);
    if (pages >= maxPages) {
      return publish(options, {
        status: "failed",
        platform: adapter.platform,
        error: new ShoppingError("UNKNOWN", `Shopping sync exceeded safety limit of ${maxPages} pages`, {
          platform: adapter.platform,
        }),
        collected,
      });
    }

    const pageNumber = pages + 1;
    await publish(options, { status: "syncing", platform: adapter.platform, page: pageNumber, collected });

    let page;
    try {
      page = await adapter.fetchOrders({ pageToken, since: options.cursor }, options.signal);
    } catch (error) {
      return failureFromError(genericAdapter, options, error, collected);
    }

    pages += 1;
    if (page.checkpoint && !checkpointCaptured) {
      checkpoint = page.checkpoint;
      checkpointCaptured = true;
    }
    if (options.signal?.aborted) return cancelled(genericAdapter, options, collected);

    const batch: UnifiedOrder[] = [];
    let boundaryReached = false;

    for (const rawOrder of page.orders) {
      if (options.signal?.aborted) return cancelled(genericAdapter, options, collected);

      let order: UnifiedOrder;
      try {
        order = await adapter.normalizeOrder(rawOrder, options.signal);
        validateAdapterOrder(genericAdapter, order);
      } catch (error) {
        return failureFromError(
          genericAdapter,
          options,
          error instanceof ShoppingError
            ? error
            : new ShoppingError("NORMALIZE_FAILED", error instanceof Error ? error.message : "Order normalization failed", {
                platform: adapter.platform,
                cause: error,
              }),
          collected,
        );
      }

      const key = getUnifiedOrderKey(order);
      if (!seenOrderKeys.has(key)) {
        seenOrderKeys.add(key);
        batch.push(order);
      }

      // Emit the boundary order once so a newly changed status is not lost.
      if (knownOrderKeys.has(key)) {
        boundaryReached = true;
        break;
      }
    }

    const emitted = await emitBatch(genericAdapter, options, batch, collected);
    collected = emitted.collected;
    if (emitted.failure) return emitted.failure;

    listTraversalComplete = boundaryReached || page.done || !page.nextPageToken;
    if (!listTraversalComplete) pageToken = page.nextPageToken;
  }

  const refreshOrderIds = [...new Set(options.refreshOrderIds ?? [])].filter(
    (platformOrderId) => !seenOrderKeys.has(`${adapter.platform}::${platformOrderId}`),
  );

  if (refreshOrderIds.length > 0 && adapter.refreshOrders) {
    if (options.signal?.aborted) return cancelled(genericAdapter, options, collected);

    let refreshedOrders: readonly UnifiedOrder[];
    try {
      refreshedOrders = await adapter.refreshOrders(refreshOrderIds, options.signal);
    } catch (error) {
      return failureFromError(genericAdapter, options, error, collected);
    }

    if (options.signal?.aborted) return cancelled(genericAdapter, options, collected);

    const refreshBatch: UnifiedOrder[] = [];
    try {
      for (const order of refreshedOrders) {
        validateAdapterOrder(genericAdapter, order);
        const key = getUnifiedOrderKey(order);
        if (seenOrderKeys.has(key)) continue;
        seenOrderKeys.add(key);
        refreshBatch.push(order);
      }
    } catch (error) {
      return failureFromError(
        genericAdapter,
        options,
        new ShoppingError("NORMALIZE_FAILED", error instanceof Error ? error.message : "Refreshed order validation failed", {
          platform: adapter.platform,
          cause: error,
        }),
        collected,
      );
    }

    const emitted = await emitBatch(genericAdapter, options, refreshBatch, collected);
    collected = emitted.collected;
    if (emitted.failure) return emitted.failure;
  }

  return publish(options, {
    status: "completed",
    platform: adapter.platform,
    pages,
    collected,
    cursor: checkpoint,
  });
}
