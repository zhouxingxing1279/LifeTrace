import type { ConnectionCheckResult, OrderPageRequest, ShoppingPlatform } from "./types";

/**
 * Lowest-level collector boundary. A source owns access to an authenticated
 * platform context (WebView, Chromium profile, official API client, etc.) and
 * returns platform-native payloads. It must not write LifeTrace business data.
 */
export interface ShoppingSource<TRawPage = unknown, TRawDetail = unknown, TRawFulfillment = unknown> {
  readonly platform: ShoppingPlatform;

  checkConnection(signal?: AbortSignal): Promise<ConnectionCheckResult>;

  fetchOrderPage(request: OrderPageRequest, signal?: AbortSignal): Promise<TRawPage>;

  fetchOrderDetail?(platformOrderId: string, signal?: AbortSignal): Promise<TRawDetail>;

  fetchFulfillment?(platformOrderId: string, signal?: AbortSignal): Promise<TRawFulfillment>;
}
