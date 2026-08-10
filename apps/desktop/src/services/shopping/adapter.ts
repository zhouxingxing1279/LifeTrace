import type {
  AdapterOrderPage,
  ConnectionCheckResult,
  OrderPageRequest,
  ShoppingPlatform,
  UnifiedOrder,
} from "./types";

/**
 * Platform-specific boundary consumed by the common sync engine.
 * Implementations may compose one or more ShoppingSource instances internally.
 */
export interface ShoppingAdapter<TRawOrder = unknown> {
  readonly platform: ShoppingPlatform;

  checkConnection(signal?: AbortSignal): Promise<ConnectionCheckResult>;

  fetchOrders(request: OrderPageRequest, signal?: AbortSignal): Promise<AdapterOrderPage<TRawOrder>>;

  normalizeOrder(rawOrder: TRawOrder, signal?: AbortSignal): Promise<UnifiedOrder> | UnifiedOrder;

  /**
   * Optionally refresh known active orders (for example pending fulfillment,
   * in-transit or refunding orders) after the newest-order traversal stops at
   * its incremental boundary. The adapter decides whether this maps to an
   * order-detail endpoint, a logistics page or another platform-native source.
   */
  refreshOrders?(platformOrderIds: readonly string[], signal?: AbortSignal): Promise<readonly UnifiedOrder[]>;
}
