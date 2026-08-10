export const KNOWN_SHOPPING_PLATFORMS = ["taobao", "jd", "pdd", "meituan"] as const;

export type KnownShoppingPlatform = (typeof KNOWN_SHOPPING_PLATFORMS)[number];
export type ShoppingPlatform = KnownShoppingPlatform | (string & {});

export type ConnectionStatus = "connected" | "auth_required" | "verification_required" | "unavailable";

export type VerificationType =
  | "login"
  | "slider"
  | "sms"
  | "qr"
  | "security_confirmation"
  | "unknown";

export interface VerificationRequirement {
  type: VerificationType;
  message?: string;
  /** Opaque, non-secret correlation identifier owned by the collector runtime. */
  correlationId?: string;
}

export interface ConnectionCheckResult {
  status: ConnectionStatus;
  verification?: VerificationRequirement;
  message?: string;
}

export type OrderStatus =
  | "pending_payment"
  | "paid"
  | "pending_fulfillment"
  | "partially_fulfilled"
  | "fulfilled"
  | "completed"
  | "cancelled"
  | "closed"
  | "refunding"
  | "refunded"
  | "unknown";

export type FulfillmentType =
  | "parcel"
  | "platform_delivery"
  | "local_delivery"
  | "pickup"
  | "virtual"
  | "none";

export type FulfillmentStatus =
  | "pending"
  | "ready"
  | "shipped"
  | "in_transit"
  | "out_for_delivery"
  | "delivered"
  | "pickup_ready"
  | "picked_up"
  | "cancelled"
  | "exception"
  | "unknown";

export type RefundStatus = "requested" | "processing" | "completed" | "rejected" | "cancelled" | "unknown";

export interface OrderItem {
  platformProductId?: string;
  name: string;
  skuName?: string;
  quantity: number;
  unitPriceMinor?: number;
  paidAmountMinor?: number;
  imageUrl?: string;
  productUrl?: string;
  categoryHint?: string;
}

export interface TrackingEvent {
  occurredAt: string;
  status: FulfillmentStatus;
  description: string;
  location?: string;
}

export interface Fulfillment {
  type: FulfillmentType;
  status: FulfillmentStatus;
  carrier?: string;
  trackingNumber?: string;
  platformFulfillmentId?: string;
  estimatedDeliveryAt?: string;
  shippedAt?: string;
  deliveredAt?: string;
  latestEvent?: TrackingEvent;
  events: TrackingEvent[];
}

export interface Refund {
  platformRefundId?: string;
  status: RefundStatus;
  amountMinor?: number;
  requestedAt?: string;
  completedAt?: string;
  itemPlatformProductIds?: string[];
}

export interface UnifiedOrder {
  platform: ShoppingPlatform;
  platformOrderId: string;
  orderedAt: string;
  paidAt?: string;
  merchantName?: string;
  status: OrderStatus;
  currency: string;
  originalAmountMinor: number;
  discountAmountMinor: number;
  shippingFeeMinor: number;
  paidAmountMinor: number;
  items: OrderItem[];
  fulfillments: Fulfillment[];
  refunds: Refund[];
  updatedAt: string;
}

/** Persistent incremental watermark. Adapters own the semantics of sourceData. */
export interface ShoppingSyncCursor {
  latestOrderId?: string;
  latestOrderTime?: string;
  sourceData?: Record<string, string | number | boolean | null>;
}

export interface OrderPageRequest {
  /** Temporary cursor used only while traversing the current sync run. */
  pageToken?: string;
  /** Persistent cursor from the previous successful sync. */
  since?: ShoppingSyncCursor;
}

export interface AdapterOrderPage<TRawOrder> {
  orders: readonly TRawOrder[];
  nextPageToken?: string;
  done: boolean;
  /** New persistent watermark. Usually emitted by the first/newest page. */
  checkpoint?: ShoppingSyncCursor;
}

export function getOrderKey(platform: ShoppingPlatform, platformOrderId: string): string {
  return `${platform}::${platformOrderId}`;
}

export function getUnifiedOrderKey(order: Pick<UnifiedOrder, "platform" | "platformOrderId">): string {
  return getOrderKey(order.platform, order.platformOrderId);
}

function assertNonNegativeInteger(value: number, field: string): void {
  if (!Number.isSafeInteger(value) || value < 0) {
    throw new TypeError(`${field} must be a non-negative safe integer in minor currency units`);
  }
}

export function validateUnifiedOrder(order: UnifiedOrder): void {
  if (!order.platform || !order.platformOrderId.trim()) {
    throw new TypeError("platform and platformOrderId are required");
  }
  if (!order.orderedAt || !order.updatedAt) {
    throw new TypeError("orderedAt and updatedAt are required");
  }
  if (!order.currency.trim()) {
    throw new TypeError("currency is required");
  }

  assertNonNegativeInteger(order.originalAmountMinor, "originalAmountMinor");
  assertNonNegativeInteger(order.discountAmountMinor, "discountAmountMinor");
  assertNonNegativeInteger(order.shippingFeeMinor, "shippingFeeMinor");
  assertNonNegativeInteger(order.paidAmountMinor, "paidAmountMinor");

  for (const [index, item] of order.items.entries()) {
    if (!item.name.trim()) throw new TypeError(`items[${index}].name is required`);
    if (!Number.isSafeInteger(item.quantity) || item.quantity <= 0) {
      throw new TypeError(`items[${index}].quantity must be a positive safe integer`);
    }
    if (item.unitPriceMinor !== undefined) assertNonNegativeInteger(item.unitPriceMinor, `items[${index}].unitPriceMinor`);
    if (item.paidAmountMinor !== undefined) assertNonNegativeInteger(item.paidAmountMinor, `items[${index}].paidAmountMinor`);
  }

  for (const [index, refund] of order.refunds.entries()) {
    if (refund.amountMinor !== undefined) assertNonNegativeInteger(refund.amountMinor, `refunds[${index}].amountMinor`);
  }
}
