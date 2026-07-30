import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";

import { ConnectionLimiter } from "../cloudflare/telegram-relay/src/lifecycle";
import {
  MAX_ACTIVE_CONNECTIONS_PER_ISOLATE,
  MAX_BYTES_PER_WINDOW,
  MAX_FRAMES_PER_WINDOW,
  MAX_MESSAGE_BYTES,
  MAX_QUEUED_FRAMES,
  MAX_SESSION_MS,
  RELAY_ACK_FRAME,
  RELAY_DATA_FRAME,
  authorizeAndRoute,
  decodeClientFrame,
  encodeClientData,
  encodeServerData,
  relayByteLength,
  toRelayBytes,
} from "../cloudflare/telegram-relay/src/policy";
import { relayConnection } from "../cloudflare/telegram-relay/src/session";

const secret = "0123456789abcdef0123456789abcdef";

function request(
  path: string,
  token = secret,
  extraHeaders: Record<string, string> = {},
) {
  return new Request(`https://relay.example${path}`, {
    headers: {
      Authorization: `Bearer ${token}`,
      Connection: "Upgrade",
      "Sec-WebSocket-Protocol": "zm-telegram-relay-v1",
      Upgrade: "websocket",
      ...extraHeaders,
    },
  });
}

describe("Telegram relay Worker policy", () => {
  it("maps only fixed Telegram DC routes to TCP 443", () => {
    expect(authorizeAndRoute(request("/v1/telegram/dc/2/main"), secret)).toEqual({
      ok: true,
      destination: { hostname: "149.154.167.51", port: 443 },
    });
    expect(authorizeAndRoute(request("/v1/telegram/dc/4/media"), secret)).toEqual({
      ok: true,
      destination: { hostname: "149.154.167.91", port: 443 },
    });
    expect(authorizeAndRoute(request("/v1/telegram/dc/203/main"), secret)).toEqual({
      ok: true,
      destination: { hostname: "91.105.192.100", port: 443 },
    });
  });

  it("rejects client-controlled destinations and unknown routes", () => {
    expect(
      authorizeAndRoute(
        request("/v1/telegram/dc/2/main?dst=1.1.1.1"),
        secret,
      ).ok,
    ).toBe(false);
    expect(authorizeAndRoute(request("/v1/telegram/dc/6/main"), secret).ok).toBe(
      false,
    );
    expect(authorizeAndRoute(request("/apiws"), secret).ok).toBe(false);
  });

  it("requires websocket upgrade and exact nontrivial bearer secret", () => {
    expect(authorizeAndRoute(request("/v1/telegram/dc/2/main", "wrong"), secret).ok).toBe(
      false,
    );
    expect(
      authorizeAndRoute(
        request("/v1/telegram/dc/2/main", "x".repeat(4_096)),
        secret,
      ).ok,
    ).toBe(false);
    expect(
      authorizeAndRoute(
        new Request("https://relay.example/v1/telegram/dc/2/main", {
          headers: { Authorization: `Bearer ${secret}` },
        }),
        secret,
      ).ok,
    ).toBe(false);
    expect(
      authorizeAndRoute(request("/v1/telegram/dc/2/main"), "too-short").ok,
    ).toBe(false);
    expect(
      authorizeAndRoute(
        new Request("http://relay.example/v1/telegram/dc/2/main", {
          headers: {
            Authorization: `Bearer ${secret}`,
            "Sec-WebSocket-Protocol": "zm-telegram-relay-v1",
            Upgrade: "websocket",
          },
        }),
        secret,
      ).ok,
    ).toBe(false);
  });

  it("uses typed relay frames and stop-and-wait acknowledgements", () => {
    expect(encodeClientData(new Uint8Array([7, 8]))).toEqual(
      new Uint8Array([RELAY_DATA_FRAME, 7, 8]),
    );
    expect(
      decodeClientFrame(
        new Uint8Array([RELAY_ACK_FRAME, 0x01, 0x02, 0x03, 0x04]),
      ),
    ).toEqual({ kind: "ack", sequence: 0x0102_0304 });
    expect(encodeServerData(0x0102_0304, new Uint8Array([9]))).toEqual(
      new Uint8Array([RELAY_DATA_FRAME, 1, 2, 3, 4, 9]),
    );
    expect(() => decodeClientFrame(new Uint8Array([0xff]))).toThrow();
  });

  it("releases connection leases exactly once", () => {
    const limiter = new ConnectionLimiter(2);
    const first = limiter.acquire();
    const second = limiter.acquire();
    expect(first).toBeDefined();
    expect(second).toBeDefined();
    expect(limiter.acquire()).toBeUndefined();
    first?.release();
    first?.release();
    expect(limiter.activeCount()).toBe(1);
    expect(limiter.acquire()).toBeDefined();
  });

  it("bounds messages and per-isolate connection admission", async () => {
    expect(MAX_MESSAGE_BYTES).toBe(64 * 1024);
    expect(MAX_ACTIVE_CONNECTIONS_PER_ISOLATE).toBeGreaterThan(0);
    expect(MAX_ACTIVE_CONNECTIONS_PER_ISOLATE).toBeLessThanOrEqual(8);
    expect(MAX_SESSION_MS).toBeLessThanOrEqual(60 * 60_000);
    expect(MAX_FRAMES_PER_WINDOW).toBeGreaterThan(0);
    expect(MAX_QUEUED_FRAMES).toBeLessThanOrEqual(MAX_FRAMES_PER_WINDOW);
    expect(MAX_BYTES_PER_WINDOW).toBeGreaterThanOrEqual(MAX_MESSAGE_BYTES);
    await expect(toRelayBytes("not binary")).rejects.toThrow();
    expect(relayByteLength(new Uint8Array([1, 2, 3]))).toBe(3);
    expect(() => relayByteLength("not binary")).toThrow();
    await expect(
      toRelayBytes(new Uint8Array(MAX_MESSAGE_BYTES + 1)),
    ).rejects.toThrow();
    await expect(toRelayBytes(new Uint8Array([1, 2, 3]))).resolves.toEqual(
      new Uint8Array([1, 2, 3]),
    );
  });

  it("relays typed frames, acknowledges egress, and cleans both sides", async () => {
    const sent: Uint8Array[] = [];
    let closed = false;
    const webSocket = new (class extends EventTarget {
      send(message: ArrayBuffer | ArrayBufferView) {
        sent.push(
          message instanceof ArrayBuffer
            ? new Uint8Array(message)
            : new Uint8Array(
                message.buffer,
                message.byteOffset,
                message.byteLength,
              ),
        );
      }
      close() {
        closed = true;
      }
    })();
    let readableController:
      | ReadableStreamDefaultController<Uint8Array>
      | undefined;
    const tcpWrites: Uint8Array[] = [];
    let socketClosed = false;
    let writerAborted = false;
    const connection = relayConnection(
      webSocket as unknown as WebSocket,
      {
        readable: new ReadableStream<Uint8Array>({
          start(controller) {
            readableController = controller;
          },
        }),
        writable: new WritableStream<Uint8Array>({
          write(chunk) {
            tcpWrites.push(new Uint8Array(chunk));
          },
          abort() {
            writerAborted = true;
          },
        }),
        async close() {
          socketClosed = true;
        },
      },
    );

    webSocket.dispatchEvent(
      new MessageEvent("message", {
        data: encodeClientData(new Uint8Array([1, 2])),
      }),
    );
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(tcpWrites).toEqual([new Uint8Array([1, 2])]);

    readableController?.enqueue(new Uint8Array([9]));
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(sent).toEqual([
      new Uint8Array([RELAY_DATA_FRAME, 0, 0, 0, 1, 9]),
    ]);
    webSocket.dispatchEvent(
      new MessageEvent("message", {
        data: new Uint8Array([RELAY_ACK_FRAME, 0, 0, 0, 1]),
      }),
    );
    readableController?.close();
    await connection;
    expect(closed).toBe(true);
    expect(socketClosed).toBe(true);
    expect(writerAborted).toBe(true);
  });

  it("has no runtime dependency, mutable destination, logging, or analytics", () => {
    const source = readFileSync(
      new URL("../cloudflare/telegram-relay/src/index.ts", import.meta.url),
      "utf8",
    );
    const sessionSource = readFileSync(
      new URL("../cloudflare/telegram-relay/src/session.ts", import.meta.url),
      "utf8",
    );
    const packageJson = JSON.parse(
      readFileSync(
        new URL("../cloudflare/telegram-relay/package.json", import.meta.url),
        "utf8",
      ),
    );
    const wrangler = readFileSync(
      new URL("../cloudflare/telegram-relay/wrangler.jsonc", import.meta.url),
      "utf8",
    );
    const gitignore = readFileSync(
      new URL("../.gitignore", import.meta.url),
      "utf8",
    );
    expect(packageJson.dependencies).toBeUndefined();
    expect(packageJson.scripts["deploy:test"]).toBeUndefined();
    expect(source).not.toMatch(/\bconsole\./u);
    expect(source).not.toMatch(/\bawait\s+fetch\s*\(/u);
    expect(source).not.toContain("searchParams.get");
    expect(source).toContain("connect(decision.destination");
    expect(source).toContain("context.waitUntil");
    expect(source.indexOf("authorizeAndRoute")).toBeLessThan(
      source.indexOf("connect(decision.destination"),
    );
    expect(sessionSource).toContain("await ackWait");
    expect(sessionSource).toContain("MAX_QUEUED_FRAMES");
    expect(wrangler).toContain('"send_metrics": false');
    expect(wrangler).toContain('"enabled": false');
    expect(gitignore).toContain(".dev.vars");
    expect(gitignore).toContain(".wrangler/");
  });
});
