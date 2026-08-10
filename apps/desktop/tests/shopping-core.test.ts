import assert from "node:assert/strict";
import test from "node:test";
import type { ShoppingAdapter } from "../src/services/shopping/adapter";
import { ShoppingError } from "../src/services/shopping/errors";
import { runShoppingSync } from "../src/services/shopping/sync-engine";
import {
  getOrderKey,
  getUnifiedOrderKey,
  validateUnifiedOrder,
  type AdapterOrderPage,
  type ConnectionCheckResult,
  type OrderPageRequest,
  type ShoppingPlatform,
  type UnifiedOrder,
} from "../src/services/shopping/types";

type RawOrder = { order: UnifiedOrder };

function makeOrder(platform: ShoppingPlatform, platformOrderId: string, overrides: Partial<UnifiedOrder> = {}): UnifiedOrder {
  return {
    platform,
    platformOrderId,
    orderedAt: "2026-08-10T10:00:00+08:00",
    merchantName: "Test Merchant",
    status: "paid",
    currency: "CNY",
    originalAmountMinor: 10000,
    discountAmountMinor: 1000,
    shippingFeeMinor: 0,
    paidAmountMinor: 9000,
    items: [{ name: "Test Item", quantity: 1, paidAmountMinor: 9000 }],
    fulfillments: [],
    refunds: [],
    updatedAt: "2026-08-10T10:01:00+08:00",
    ...overrides,
  };
}

class FakeAdapter implements ShoppingAdapter<RawOrder> {
  readonly requests: OrderPageRequest[] = [];
  readonly platform: ShoppingPlatform;
  private readonly pages: AdapterOrderPage<RawOrder>[];
  private readonly connection: ConnectionCheckResult;
  private readonly errorsByFetchIndex: Map<number, unknown>;

  constructor(options: {
    platform?: ShoppingPlatform;
    pages?: AdapterOrderPage<RawOrder>[];
    connection?: ConnectionCheckResult;
    errorsByFetchIndex?: Map<number, unknown>;
  } = {}) {
    this.platform = options.platform ?? "jd";
    this.pages = options.pages ?? [];
    this.connection = options.connection ?? { status: "connected" };
    this.errorsByFetchIndex = options.errorsByFetchIndex ?? new Map();
  }

  async checkConnection(): Promise<ConnectionCheckResult> {
    return this.connection;
  }

  async fetchOrders(request: OrderPageRequest): Promise<AdapterOrderPage<RawOrder>> {
    const index = this.requests.length;
    this.requests.push(request);
    const error = this.errorsByFetchIndex.get(index);
    if (error) throw error;
    return this.pages[index] ?? { orders: [], done: true };
  }

  normalizeOrder(rawOrder: RawOrder): UnifiedOrder {
    return rawOrder.order;
  }
}

test("builds stable order keys without cross-platform collisions", () => {
  assert.equal(getOrderKey("jd", "123"), "jd::123");
  assert.equal(getUnifiedOrderKey(makeOrder("jd", "123")), "jd::123");
  assert.notEqual(getOrderKey("jd", "123"), getOrderKey("taobao", "123"));
});

test("unified order supports multiple items, fulfillments and refunds", () => {
  const order = makeOrder("taobao", "multi", {
    items: [
      { platformProductId: "p1", name: "Keyboard", quantity: 1, paidAmountMinor: 39900 },
      { platformProductId: "p2", name: "Cable", quantity: 2, paidAmountMinor: 2000 },
    ],
    fulfillments: [
      {
        type: "parcel",
        status: "in_transit",
        carrier: "ZTO",
        trackingNumber: "TEST123",
        events: [{ occurredAt: "2026-08-10T12:00:00+08:00", status: "in_transit", description: "In transit" }],
      },
      { type: "pickup", status: "pickup_ready", events: [] },
    ],
    refunds: [
      { platformRefundId: "r1", status: "processing", amountMinor: 1000 },
      { platformRefundId: "r2", status: "completed", amountMinor: 500 },
    ],
  });

  assert.doesNotThrow(() => validateUnifiedOrder(order));
  assert.equal(order.items.length, 2);
  assert.equal(order.fulfillments.length, 2);
  assert.equal(order.refunds.length, 2);
});

test("rejects non-integer minor currency amounts", () => {
  const order = makeOrder("jd", "bad-money", { paidAmountMinor: 99.5 });
  assert.throws(() => validateUnifiedOrder(order), /minor currency units/);
});

test("syncs multiple pages, advances page token and returns newest checkpoint", async () => {
  const firstCheckpoint = { latestOrderId: "3", latestOrderTime: "2026-08-10T10:00:00+08:00" };
  const adapter = new FakeAdapter({
    pages: [
      {
        orders: [{ order: makeOrder("jd", "3") }, { order: makeOrder("jd", "2") }],
        nextPageToken: "page-2",
        done: false,
        checkpoint: firstCheckpoint,
      },
      { orders: [{ order: makeOrder("jd", "1") }], done: true },
    ],
  });
  const received: string[] = [];

  const result = await runShoppingSync(adapter, {
    cursor: { latestOrderId: "old" },
    onBatch: (orders) => received.push(...orders.map(getUnifiedOrderKey)),
  });

  assert.equal(result.status, "completed");
  if (result.status !== "completed") return;
  assert.equal(result.pages, 2);
  assert.equal(result.collected, 3);
  assert.deepEqual(result.cursor, firstCheckpoint);
  assert.deepEqual(received, ["jd::3", "jd::2", "jd::1"]);
  assert.equal(adapter.requests[0]?.pageToken, undefined);
  assert.equal(adapter.requests[1]?.pageToken, "page-2");
  assert.equal(adapter.requests[0]?.since?.latestOrderId, "old");
  assert.equal(adapter.requests[1]?.since?.latestOrderId, "old");
});

test("stops at a known order boundary without emitting older orders", async () => {
  const adapter = new FakeAdapter({
    pages: [
      {
        orders: [
          { order: makeOrder("jd", "3") },
          { order: makeOrder("jd", "2") },
          { order: makeOrder("jd", "1") },
        ],
        nextPageToken: "should-not-be-used",
        done: false,
      },
    ],
  });
  const received: string[] = [];

  const result = await runShoppingSync(adapter, {
    knownOrderKeys: ["jd::2"],
    onBatch: (orders) => received.push(...orders.map(getUnifiedOrderKey)),
  });

  assert.equal(result.status, "completed");
  assert.deepEqual(received, ["jd::3"]);
  assert.equal(adapter.requests.length, 1);
});

test("deduplicates repeated orders inside one sync run", async () => {
  const repeated = makeOrder("pdd", "same");
  const adapter = new FakeAdapter({
    platform: "pdd",
    pages: [{ orders: [{ order: repeated }, { order: repeated }], done: true }],
  });
  const received: UnifiedOrder[] = [];

  const result = await runShoppingSync(adapter, { onBatch: (orders) => received.push(...orders) });

  assert.equal(result.status, "completed");
  assert.equal(received.length, 1);
});

test("pauses before fetching when connection requires user verification", async () => {
  const adapter = new FakeAdapter({
    connection: {
      status: "verification_required",
      verification: { type: "slider", message: "Complete the slider in the original browser session" },
    },
  });

  const result = await runShoppingSync(adapter);

  assert.equal(result.status, "verification_required");
  if (result.status !== "verification_required") return;
  assert.equal(result.verification.type, "slider");
  assert.equal(adapter.requests.length, 0);
});

test("pauses immediately when verification appears during pagination", async () => {
  const verificationError = new ShoppingError("VERIFICATION_REQUIRED", "Verification required", {
    platform: "jd",
    verification: { type: "sms" },
  });
  const adapter = new FakeAdapter({
    pages: [
      { orders: [{ order: makeOrder("jd", "2") }], nextPageToken: "page-2", done: false },
    ],
    errorsByFetchIndex: new Map([[1, verificationError]]),
  });
  const received: string[] = [];

  const result = await runShoppingSync(adapter, {
    onBatch: (orders) => received.push(...orders.map(getUnifiedOrderKey)),
  });

  assert.equal(result.status, "verification_required");
  if (result.status !== "verification_required") return;
  assert.equal(result.verification.type, "sms");
  assert.equal(result.collected, 1);
  assert.deepEqual(received, ["jd::2"]);
  assert.equal(adapter.requests.length, 2);
});

test("returns auth-required pause without hitting the order source", async () => {
  const adapter = new FakeAdapter({ connection: { status: "auth_required" } });

  const result = await runShoppingSync(adapter);

  assert.deepEqual(result, { status: "paused", platform: "jd", reason: "auth_required", collected: 0 });
  assert.equal(adapter.requests.length, 0);
});

test("isolates an unavailable platform as a structured failure", async () => {
  const adapter = new FakeAdapter({ platform: "meituan", connection: { status: "unavailable", message: "source offline" } });

  const result = await runShoppingSync(adapter);

  assert.equal(result.status, "failed");
  if (result.status !== "failed") return;
  assert.equal(result.platform, "meituan");
  assert.equal(result.error.code, "SOURCE_UNAVAILABLE");
  assert.equal(result.error.retryable, true);
});

test("cancels after a committed batch without fetching another page", async () => {
  const controller = new AbortController();
  const adapter = new FakeAdapter({
    pages: [
      { orders: [{ order: makeOrder("jd", "2") }], nextPageToken: "page-2", done: false },
      { orders: [{ order: makeOrder("jd", "1") }], done: true },
    ],
  });

  const result = await runShoppingSync(adapter, {
    signal: controller.signal,
    onBatch: () => controller.abort(),
  });

  assert.deepEqual(result, { status: "paused", platform: "jd", reason: "cancelled", collected: 1 });
  assert.equal(adapter.requests.length, 1);
});

test("normalization validation failures use NORMALIZE_FAILED", async () => {
  const adapter = new FakeAdapter({
    pages: [{ orders: [{ order: makeOrder("jd", "bad", { paidAmountMinor: 1.25 }) }], done: true }],
  });

  const result = await runShoppingSync(adapter);

  assert.equal(result.status, "failed");
  if (result.status !== "failed") return;
  assert.equal(result.error.code, "NORMALIZE_FAILED");
});

test("adapter cannot normalize an order for another platform", async () => {
  const adapter = new FakeAdapter({
    platform: "jd",
    pages: [{ orders: [{ order: makeOrder("taobao", "wrong-platform") }], done: true }],
  });

  const result = await runShoppingSync(adapter);

  assert.equal(result.status, "failed");
  if (result.status !== "failed") return;
  assert.equal(result.error.code, "NORMALIZE_FAILED");
  assert.match(result.error.message, /adapter platform jd/);
});
