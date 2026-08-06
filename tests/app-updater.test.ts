import assert from "node:assert/strict";
import { afterEach, beforeEach, mock, test } from "node:test";

type DownloadEvent =
  | { event: "Started"; data: { contentLength?: number } }
  | { event: "Progress"; data: { chunkLength: number } }
  | { event: "Finished" };

type FakeUpdate = {
  version: string;
  currentVersion: string;
  body?: string;
  date?: string;
  downloadAndInstall: (
    onEvent?: (event: DownloadEvent) => void,
  ) => Promise<void>;
};

const updaterModule = await import("../src/services/appUpdater");

let checkMock: ReturnType<typeof mock.fn<() => Promise<FakeUpdate | null>>>;
let relaunchMock: ReturnType<typeof mock.fn<() => Promise<void>>>;
let service: ReturnType<typeof updaterModule.createAppUpdaterService>;

beforeEach(() => {
  checkMock = mock.fn<() => Promise<FakeUpdate | null>>();
  relaunchMock = mock.fn<() => Promise<void>>();
  service = updaterModule.createAppUpdaterService({
    isTauri: () => true,
    check: checkMock as unknown as () => Promise<never>,
    relaunch: relaunchMock as () => Promise<void>,
  });
});

afterEach(() => {
  mock.reset();
});

test("non-Tauri environments refuse to start an update check", async () => {
  const nonTauri = updaterModule.createAppUpdaterService({
    isTauri: () => false,
    check: checkMock as unknown as () => Promise<never>,
    relaunch: relaunchMock as () => Promise<void>,
  });
  await assert.rejects(
    async () => nonTauri.checkForAppUpdate(),
    /不支持自动更新/,
  );
  assert.equal(checkMock.mock.calls.length, 0);

  delete (globalThis as Record<string, unknown>).window;
  await assert.rejects(
    async () => updaterModule.checkForAppUpdate(),
    /不支持自动更新/,
  );
});

test("checkForAppUpdate returns null when no update is available", async () => {
  checkMock.mock.mockImplementation(async () => null);
  const result = await service.checkForAppUpdate();
  assert.equal(result, null);
  assert.equal(checkMock.mock.calls.length, 1);
});

test("checkForAppUpdate maps a failed check to a readable error", async () => {
  checkMock.mock.mockImplementation(async () => {
    throw new Error("error sending request for url (https://github.com/...)");
  });
  await assert.rejects(() => service.checkForAppUpdate(), /网络连接失败/);
});

test("concurrent checks share a single in-flight task", async () => {
  let calls = 0;
  checkMock.mock.mockImplementation(async () => {
    calls += 1;
    await new Promise((resolve) => setTimeout(resolve, 20));
    return null;
  });
  const [first, second] = await Promise.all([
    service.checkForAppUpdate(),
    service.checkForAppUpdate(),
  ]);
  assert.equal(first, null);
  assert.equal(second, null);
  assert.equal(calls, 1);
});

test("download Started event initializes progress with the known total", async () => {
  const update = await createUpdate([
    { event: "Started", data: { contentLength: 100 } },
    { event: "Finished" },
  ]);
  const states: string[] = [];
  await update.downloadAndInstall((state) => {
    if (state.status === "downloading") {
      states.push(
        `${state.downloadedBytes}/${state.totalBytes}/${state.percentage}`,
      );
    }
  });
  assert.deepEqual(states, ["0/100/0"]);
});

test("download Progress events accumulate bytes and compute percentage", async () => {
  const update = await createUpdate([
    { event: "Started", data: { contentLength: 200 } },
    { event: "Progress", data: { chunkLength: 50 } },
    { event: "Progress", data: { chunkLength: 50 } },
    { event: "Finished" },
  ]);
  const states: string[] = [];
  await update.downloadAndInstall((state) => {
    if (state.status === "downloading") {
      states.push(
        `${state.downloadedBytes}/${state.totalBytes}/${state.percentage}`,
      );
    }
  });
  assert.deepEqual(states, ["0/200/0", "50/200/25", "100/200/50"]);
});

test("download with unknown total keeps percentage null", async () => {
  const update = await createUpdate([
    { event: "Started", data: {} },
    { event: "Progress", data: { chunkLength: 12 } },
    { event: "Finished" },
  ]);
  const states: string[] = [];
  await update.downloadAndInstall((state) => {
    if (state.status === "downloading") {
      states.push(
        `${state.downloadedBytes}/${state.totalBytes}/${state.percentage}`,
      );
    }
  });
  assert.deepEqual(states, ["0/null/null", "12/null/null"]);
});

test("download finishes into installing state and relaunches the app", async () => {
  const update = await createUpdate([
    { event: "Started", data: { contentLength: 100 } },
    { event: "Finished" },
  ]);
  const states: string[] = [];
  await update.downloadAndInstall((state) => states.push(state.status));
  assert.ok(states.includes("installing"));
  assert.equal(relaunchMock.mock.calls.length, 1);
});

test("concurrent downloads start only one update task", async () => {
  const update = await createUpdate([
    { event: "Started", data: { contentLength: 100 } },
    { event: "Finished" },
  ]);
  await Promise.all([
    update.downloadAndInstall(),
    update.downloadAndInstall(),
  ]);
  assert.equal(relaunchMock.mock.calls.length, 1);
  assert.equal(checkMock.mock.calls.length, 1);
});

test("pure helpers behave correctly", () => {
  (globalThis as Record<string, unknown>).window = { __TAURI_INTERNALS__: {} };
  assert.equal(updaterModule.isTauriDesktopRuntime(), true);
  delete (globalThis as Record<string, unknown>).window;
  assert.equal(updaterModule.isTauriDesktopRuntime(), false);
  assert.equal(updaterModule.calculateDownloadProgress(50, 200), 25);
  assert.equal(updaterModule.calculateDownloadProgress(0, 0), null);
  assert.equal(updaterModule.calculateDownloadProgress(10, null), null);
  assert.equal(updaterModule.normalizeUpdateError(null), "更新失败，请稍后重试。");
  assert.equal(
    updaterModule.normalizeUpdateError(new Error("signature verification failed")),
    "更新签名校验失败，安装包可能被篡改或签名密钥不匹配。",
  );
  assert.equal(
    updaterModule.normalizeUpdateError(new Error("download failed: disk full")),
    "下载或写入安装包失败，请检查磁盘空间后重试。",
  );
});

test("auto check is skipped in dev or outside Tauri", () => {
  assert.equal(
    updaterModule.shouldAutoCheckForUpdate({ isTauri: true, isDev: false }),
    true,
  );
  assert.equal(
    updaterModule.shouldAutoCheckForUpdate({ isTauri: true, isDev: true }),
    false,
  );
  assert.equal(
    updaterModule.shouldAutoCheckForUpdate({ isTauri: false, isDev: false }),
    false,
  );
});

test("single flight shares the in-flight promise", async () => {
  let runs = 0;
  const flight = updaterModule.createSingleFlight<number>();
  const task = async () => {
    runs += 1;
    await new Promise((resolve) => setTimeout(resolve, 10));
    return 42;
  };
  const [a, b] = await Promise.all([flight.run(task), flight.run(task)]);
  assert.equal(a, 42);
  assert.equal(b, 42);
  assert.equal(runs, 1);
});

async function createUpdate(events: DownloadEvent[]) {
  const fakeUpdate = {
    version: "0.2.2",
    currentVersion: "0.2.1",
    body: "更新说明",
    date: "2026-08-07T00:00:00Z",
    downloadAndInstall: mock.fn(
      async (onEvent?: (event: DownloadEvent) => void) => {
        for (const event of events) onEvent?.(event);
      },
    ),
  };
  checkMock.mock.mockImplementation(async () => fakeUpdate);
  const available = await service.checkForAppUpdate();
  assert.ok(available);
  return available;
}
