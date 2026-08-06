import assert from "node:assert/strict";
import test from "node:test";
import { RegistrationApi } from "../web-client/src/registration";

function response(payload: unknown, status = 200) {
  return new Response(JSON.stringify(payload), { status, headers: { "content-type": "application/json" } });
}

test("registration capabilities are loaded from the authentication service", async () => {
  let capturedUrl = "";
  const api = new RegistrationApi(async (input, init) => {
    capturedUrl = String(input);
    assert.equal(init?.credentials, "include");
    return response({ registrationMode: "invite", passwordMinLength: 12, passwordMaxBytes: 256, webSessionEnabled: true });
  });
  const capabilities = await api.capabilities();
  assert.equal(capturedUrl, "/api/v1/auth/capabilities");
  assert.equal(capabilities.registrationMode, "invite");
  assert.equal(capabilities.passwordMinLength, 12);
});

test("browser registration creates a cookie session with required scopes", async () => {
  let captured: { url: string; init?: RequestInit } | null = null;
  const api = new RegistrationApi(async (input, init) => {
    captured = { url: String(input), init };
    return response({ user: { id: "u", email: "new@example.com", displayName: "新用户" }, session: { id: "s", deviceId: "d" }, csrfToken: "csrf" }, 201);
  });
  const session = await api.register({
    email: " new@example.com ",
    password: "a sufficiently long password",
    displayName: " 新用户 ",
    inviteToken: " invite-token ",
    publicDevice: true,
  });
  assert.equal(session.user.email, "new@example.com");
  assert.equal(captured?.url, "/api/v1/web/session/register");
  assert.equal(captured?.init?.credentials, "include");
  const body = JSON.parse(String(captured?.init?.body)) as Record<string, unknown>;
  assert.equal(body.email, "new@example.com");
  assert.equal(body.displayName, "新用户");
  assert.equal(body.inviteToken, "invite-token");
  assert.equal(body.publicDevice, true);
  const requestedScopes = body.requestedScopes as string[];
  for (const scope of ["sync:write", "finance:write", "notes:write", "english:write", "files:write", "account:write", "devices:write", "sessions:write"]) assert.ok(requestedScopes.includes(scope), scope);
});
