export const RELAY_PROTOCOL = "zm-telegram-relay-v1";
export const MAX_MESSAGE_BYTES = 64 * 1024;
export const MAX_ACTIVE_CONNECTIONS_PER_ISOLATE = 4;
export const MAX_QUEUED_BYTES = 2 * 1024 * 1024;
export const MAX_QUEUED_FRAMES = 128;
export const MAX_FRAMES_PER_WINDOW = 128;
export const MAX_BYTES_PER_WINDOW = 32 * 1024 * 1024;
export const RATE_WINDOW_MS = 1_000;
export const CONNECT_TIMEOUT_MS = 10_000;
export const WRITE_TIMEOUT_MS = 30_000;
export const ACK_TIMEOUT_MS = 75_000;
export const IDLE_TIMEOUT_MS = 5 * 60_000;
export const MAX_SESSION_MS = 60 * 60_000;
export const RELAY_DATA_FRAME = 0x01;
export const RELAY_ACK_FRAME = 0x02;

export interface RelayDestination {
  hostname: string;
  port: 443;
}

export type RelayDecision =
  | { ok: true; destination: RelayDestination }
  | { ok: false; status: 400 | 401 | 404 | 405 | 426 | 503 };

const DESTINATIONS: Readonly<Record<string, RelayDestination>> = Object.freeze({
  "1": Object.freeze({ hostname: "149.154.175.50", port: 443 }),
  "2": Object.freeze({ hostname: "149.154.167.51", port: 443 }),
  "3": Object.freeze({ hostname: "149.154.175.100", port: 443 }),
  "4": Object.freeze({ hostname: "149.154.167.91", port: 443 }),
  "5": Object.freeze({ hostname: "149.154.171.5", port: 443 }),
  "203": Object.freeze({ hostname: "91.105.192.100", port: 443 }),
});

const ROUTE = /^\/v1\/telegram\/dc\/(1|2|3|4|5|203)\/(main|media)$/;

function constantTimeEqual(left: string, right: string): boolean {
  const encoder = new TextEncoder();
  const leftBytes = encoder.encode(left);
  const rightBytes = encoder.encode(right);
  const length = Math.max(leftBytes.length, rightBytes.length);
  let difference = leftBytes.length ^ rightBytes.length;
  for (let index = 0; index < length; index += 1) {
    difference |=
      (leftBytes[index] ?? 0) ^ (rightBytes[index] ?? 0);
  }
  return difference === 0;
}

export function authorizeAndRoute(
  request: Request,
  configuredSecret: string,
): RelayDecision {
  if (request.method !== "GET") {
    return { ok: false, status: 405 };
  }
  if ((request.headers.get("Upgrade") ?? "").toLowerCase() !== "websocket") {
    return { ok: false, status: 426 };
  }
  if (request.headers.get("Sec-WebSocket-Protocol") !== RELAY_PROTOCOL) {
    return { ok: false, status: 426 };
  }
  const url = new URL(request.url);
  if (url.protocol !== "https:") {
    return { ok: false, status: 426 };
  }
  if (
    configuredSecret.length < 32 ||
    configuredSecret.length > 256 ||
    !/^[\u0021-\u007e]+$/u.test(configuredSecret)
  ) {
    return { ok: false, status: 503 };
  }
  const authorization = request.headers.get("Authorization") ?? "";
  const prefix = "Bearer ";
  const presentedSecret = authorization.startsWith(prefix)
    ? authorization.slice(prefix.length)
    : "";
  if (
    !authorization.startsWith(prefix) ||
    presentedSecret.length > 256 ||
    !constantTimeEqual(presentedSecret, configuredSecret)
  ) {
    return { ok: false, status: 401 };
  }

  if (url.search !== "" || url.hash !== "") {
    return { ok: false, status: 400 };
  }
  const route = ROUTE.exec(url.pathname);
  if (!route) {
    return { ok: false, status: 404 };
  }
  return { ok: true, destination: DESTINATIONS[route[1]] };
}

export async function toRelayBytes(
  data: unknown,
  maximum = MAX_MESSAGE_BYTES,
): Promise<Uint8Array> {
  const length = relayByteLength(data);
  if (length > maximum) {
    throw new Error("message_too_large");
  }
  let bytes: Uint8Array;
  if (data instanceof ArrayBuffer) {
    bytes = new Uint8Array(data);
  } else if (ArrayBuffer.isView(data)) {
    bytes = new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
  } else if (data instanceof Blob) {
    bytes = new Uint8Array(await data.arrayBuffer());
  } else {
    throw new Error("binary_message_required");
  }
  if (bytes.byteLength > maximum) {
    throw new Error("message_too_large");
  }
  return new Uint8Array(bytes);
}

export function toRelayBytesSync(
  data: unknown,
  maximum = MAX_MESSAGE_BYTES,
): Uint8Array {
  const length = relayByteLength(data);
  if (length > maximum) {
    throw new Error("message_too_large");
  }
  if (data instanceof ArrayBuffer) {
    return new Uint8Array(data);
  }
  if (ArrayBuffer.isView(data)) {
    return new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
  }
  throw new Error("binary_array_message_required");
}

export function relayByteLength(data: unknown): number {
  if (data instanceof ArrayBuffer) {
    return data.byteLength;
  }
  if (ArrayBuffer.isView(data)) {
    return data.byteLength;
  }
  if (data instanceof Blob) {
    return data.size;
  }
  throw new Error("binary_message_required");
}

export function encodeClientData(payload: Uint8Array): Uint8Array {
  if (payload.byteLength === 0 || payload.byteLength > MAX_MESSAGE_BYTES) {
    throw new Error("invalid_client_payload");
  }
  const frame = new Uint8Array(payload.byteLength + 1);
  frame[0] = RELAY_DATA_FRAME;
  frame.set(payload, 1);
  return frame;
}

export function decodeClientFrame(
  frame: Uint8Array,
):
  | { kind: "data"; payload: Uint8Array }
  | { kind: "ack"; sequence: number } {
  if (frame[0] === RELAY_DATA_FRAME && frame.byteLength > 1) {
    return { kind: "data", payload: frame.slice(1) };
  }
  if (frame[0] === RELAY_ACK_FRAME && frame.byteLength === 5) {
    const view = new DataView(frame.buffer, frame.byteOffset, frame.byteLength);
    return { kind: "ack", sequence: view.getUint32(1) };
  }
  throw new Error("invalid_relay_frame");
}

export function encodeServerData(
  sequence: number,
  payload: Uint8Array,
): Uint8Array {
  if (payload.byteLength === 0 || payload.byteLength > MAX_MESSAGE_BYTES) {
    throw new Error("invalid_server_payload");
  }
  const frame = new Uint8Array(payload.byteLength + 5);
  const view = new DataView(frame.buffer);
  frame[0] = RELAY_DATA_FRAME;
  view.setUint32(1, sequence);
  frame.set(payload, 5);
  return frame;
}
