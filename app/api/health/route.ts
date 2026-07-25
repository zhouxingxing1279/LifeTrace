const healthPayload = () => ({
  ok: true,
  service: "lifetrace-upload",
  checkedAt: new Date().toISOString(),
});

const headers = {
  "cache-control": "no-store, max-age=0",
  "content-type": "application/json; charset=utf-8",
};

export function GET() {
  return new Response(JSON.stringify(healthPayload()), { status: 200, headers });
}

export function HEAD() {
  return new Response(null, { status: 200, headers });
}
