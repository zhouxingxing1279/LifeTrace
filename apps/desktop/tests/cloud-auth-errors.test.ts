import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

import { cloudAuthErrorMessage, rawCloudAuthErrorMessage } from "../src/services/cloudAuthError";

class HttpLikeError extends Error {
  constructor(message: string, readonly status: number, readonly code?: string) {
    super(message);
  }
}

test("cloud auth keeps native string failures instead of collapsing to generic login failure", () => {
  assert.equal(rawCloudAuthErrorMessage("Windows Credential Manager write failed: access denied"), "Windows Credential Manager write failed: access denied");
  assert.match(
    cloudAuthErrorMessage("Windows Credential Manager write failed: access denied", "登录失败"),
    /Windows 安全登录凭据/,
  );
});

test("cloud auth maps stable HTTP failures to actionable Chinese messages", () => {
  assert.equal(cloudAuthErrorMessage(new HttpLikeError("bad", 401, "auth_invalid"), "登录失败"), "邮箱或密码错误");
  assert.equal(cloudAuthErrorMessage(new HttpLikeError("locked", 423, "auth_user_locked"), "登录失败"), "账号暂时锁定，请稍后再试");
  assert.equal(cloudAuthErrorMessage(new HttpLikeError("limited", 429, "auth_rate_limited"), "登录失败"), "登录尝试过多，请稍后再试");
  assert.equal(cloudAuthErrorMessage(new HttpLikeError("down", 503), "登录失败"), "云端认证服务暂时不可用，请稍后重试");
});

test("cloud auth turns transport failures into a network diagnosis", () => {
  assert.equal(
    cloudAuthErrorMessage(new TypeError("Failed to fetch"), "登录失败"),
    "无法连接 LifeTrace 云端，请检查服务器地址和网络连接",
  );
});

test("desktop auth reports the same client version as package metadata", () => {
  const here = path.dirname(fileURLToPath(import.meta.url));
  const root = path.resolve(here, "..");
  const packageJson = JSON.parse(readFileSync(path.join(root, "package.json"), "utf8")) as { version: string };
  const source = readFileSync(path.join(root, "src", "services", "cloudAuth.ts"), "utf8");
  assert.match(source, new RegExp(`const CLIENT_VERSION = ["']${packageJson.version.replaceAll(".", "\\.")}["']`));
});
