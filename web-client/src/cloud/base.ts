// LifeTrace Cloud API base URL.
//
// Default: talk to the LifeTrace Cloud listener on the same host the page is
// served from (port 8787), so the web-session cookie stays same-site and is
// sent on every API request. Set VITE_LIFETRACE_CLOUD_URL to override; the
// page origin must then be listed in the backend CORS allowlist
// (see deploy/cloud/docker-compose.local.yml).
export const API_BASE: string = (
  import.meta.env.VITE_LIFETRACE_CLOUD_URL
  ?? (typeof window !== "undefined"
    ? `${window.location.protocol}//${window.location.hostname}:8787`
    : "")
).toString().replace(/\/+$/, "");
