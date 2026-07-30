import { connect } from "cloudflare:sockets";

import { ConnectionLimiter } from "./lifecycle";
import {
  CONNECT_TIMEOUT_MS,
  MAX_ACTIVE_CONNECTIONS_PER_ISOLATE,
  RELAY_PROTOCOL,
  authorizeAndRoute,
} from "./policy";
import { relayConnection } from "./session";

interface Env {
  RELAY_TOKEN: string;
}

const connectionLimiter = new ConnectionLimiter(
  MAX_ACTIVE_CONNECTIONS_PER_ISOLATE,
);
type CloudflareSocket = ReturnType<typeof connect>;

function withTimeout<T>(
  promise: Promise<T>,
  milliseconds: number,
  code: string,
): Promise<T> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(code)), milliseconds);
    promise.then(
      (value) => {
        clearTimeout(timer);
        resolve(value);
      },
      (error) => {
        clearTimeout(timer);
        reject(error);
      },
    );
  });
}

export default {
  async fetch(
    request: Request,
    env: Env,
    context: ExecutionContext,
  ): Promise<Response> {
    const decision = authorizeAndRoute(request, env.RELAY_TOKEN ?? "");
    if (!decision.ok) {
      return new Response("Relay request rejected", {
        status: decision.status,
        headers: { "Cache-Control": "no-store" },
      });
    }
    const lease = connectionLimiter.acquire();
    if (!lease) {
      return new Response("Relay is busy", {
        status: 503,
        headers: { "Cache-Control": "no-store" },
      });
    }

    let socket: CloudflareSocket | undefined;
    try {
      socket = connect(decision.destination, {
        allowHalfOpen: false,
        secureTransport: "off",
      });
      await withTimeout(
        socket.opened,
        CONNECT_TIMEOUT_MS,
        "tcp_connect_timeout",
      );

      const pair = new WebSocketPair();
      const client = pair[0];
      const server = pair[1];
      server.accept({ allowHalfOpen: true });
      context.waitUntil(
        relayConnection(server, socket)
          .catch(() => undefined)
          .finally(() => {
            lease.release();
          }),
      );
      return new Response(null, {
        status: 101,
        webSocket: client,
        headers: {
          "Sec-WebSocket-Protocol": RELAY_PROTOCOL,
          "Cache-Control": "no-store",
        },
      });
    } catch {
      try {
        await socket?.close();
      } catch {
        // Connection setup failed before ownership transfer.
      }
      lease.release();
      return new Response("Relay connection failed", {
        status: 502,
        headers: { "Cache-Control": "no-store" },
      });
    }
  },
} satisfies ExportedHandler<Env>;
