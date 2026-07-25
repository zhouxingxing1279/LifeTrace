import { readFileSync } from "node:fs";
import http from "node:http";
import https from "node:https";
import path from "node:path";
import process from "node:process";

const projectRoot = process.cwd();
const certificatePath = path.join(projectRoot, ".local-certs", "lifetrace-local.pfx");
const listenPort = Number(process.env.LIFETRACE_HTTPS_PORT || 3443);
const targetPort = Number(process.env.LIFETRACE_HTTP_PORT || 3103);
const unavailableMessage = "\u6052\u5e8f\u672c\u5730\u670d\u52a1\u6682\u65f6\u65e0\u6cd5\u8fde\u63a5\uff0c\u8bf7\u786e\u8ba4\u7535\u8111\u7aef\u670d\u52a1\u6b63\u5728\u8fd0\u884c\u3002";

const server = https.createServer({
  pfx: readFileSync(certificatePath),
}, (request, response) => {
  let settled = false;
  const fail = () => {
    if (settled || response.writableEnded) return;
    settled = true;
    if (!response.headersSent) {
      response.writeHead(502, { "content-type": "text/plain; charset=utf-8" });
    }
    response.end(unavailableMessage);
  };

  const proxy = http.request({
    hostname: "127.0.0.1",
    port: targetPort,
    method: request.method,
    path: request.url,
    headers: {
      ...request.headers,
      host: request.headers.host,
      "x-forwarded-proto": "https",
      "x-forwarded-host": request.headers.host,
    },
  }, (upstream) => {
    settled = true;
    response.writeHead(upstream.statusCode ?? 502, upstream.headers);
    upstream.pipe(response);
  });

  proxy.on("error", fail);
  request.on("aborted", () => proxy.destroy());
  response.on("error", () => proxy.destroy());
  request.pipe(proxy);
});

server.listen(listenPort, "0.0.0.0", () => {
  console.log(`Life trace local HTTPS is available on port ${listenPort}.`);
});
