import { REQUESTED_SCOPES } from "./core";

// Auth protocol accepts a requested scope set. The Web client needs these
// additional scopes for device/session management and user.preference budgets.
const scopes = REQUESTED_SCOPES as unknown as string[];
for (const scope of [
  "account:read",
  "account:write",
  "devices:read",
  "devices:write",
  "sessions:read",
  "sessions:write",
]) {
  if (!scopes.includes(scope)) scopes.push(scope);
}
