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
}
