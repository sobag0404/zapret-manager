import {
  ACK_TIMEOUT_MS,
  IDLE_TIMEOUT_MS,
  MAX_BYTES_PER_WINDOW,
  MAX_FRAMES_PER_WINDOW,
  MAX_MESSAGE_BYTES,
  MAX_QUEUED_BYTES,
  MAX_QUEUED_FRAMES,
  MAX_SESSION_MS,
  RATE_WINDOW_MS,
  WRITE_TIMEOUT_MS,
  decodeClientFrame,
  encodeServerData,
  relayByteLength,
  toRelayBytesSync,
} from "./policy";

export interface RelayTcpSocket {
  readable: ReadableStream<Uint8Array>;
  writable: WritableStream<Uint8Array>;
  close(): Promise<void>;
}

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

function closeQuietly(webSocket: WebSocket, code = 1011): void {
  try {
    webSocket.close(code, "relay closed");
  } catch {
    // The peer may already be closed.
  }
}

export async function relayConnection(
  server: WebSocket,
  socket: RelayTcpSocket,
): Promise<void> {
  const reader = socket.readable.getReader();
  const writer = socket.writable.getWriter();
  let queuedBytes = 0;
  let queuedFrames = 0;
  let closed = false;
  let lastActivity = Date.now();
  let ingressWindowStarted = Date.now();
  let ingressFrames = 0;
  let ingressBytes = 0;
  let egressWindowStarted = Date.now();
  let egressFrames = 0;
  let egressBytes = 0;
  let nextSequence = 1;
  let pendingAck:
    | {
        sequence: number;
        resolve: () => void;
        reject: (error: Error) => void;
      }
    | undefined;
  let writeQueue = Promise.resolve();
  let cleanup: Promise<void> | undefined;

  const finish = (): Promise<void> => {
    if (cleanup) return cleanup;
    closed = true;
    clearInterval(idleTimer);
    clearTimeout(sessionTimer);
    pendingAck?.reject(new Error("relay_closed"));
    pendingAck = undefined;
    closeQuietly(server, 1000);
    cleanup = (async () => {
      let socketClose: Promise<void>;
      try {
        socketClose = socket.close().catch(() => undefined);
      } catch {
        socketClose = Promise.resolve();
      }
      try {
        await writer.abort(new Error("relay_closed"));
      } catch {
        // A completed or already-aborted writer needs no further cleanup.
      }
      await socketClose;
    })();
    return cleanup;
  };

  const idleTimer = setInterval(() => {
    if (Date.now() - lastActivity > IDLE_TIMEOUT_MS) {
      void finish();
    }
  }, 30_000);
  const sessionTimer = setTimeout(() => {
    void finish();
  }, MAX_SESSION_MS);

  server.addEventListener("message", (event) => {
    if (closed) return;
    try {
      const wireLength = relayByteLength(event.data);
      if (wireLength > MAX_MESSAGE_BYTES + 1) {
        throw new Error("message_limit");
      }
      const now = Date.now();
      if (now - ingressWindowStarted >= RATE_WINDOW_MS) {
        ingressWindowStarted = now;
        ingressFrames = 0;
        ingressBytes = 0;
      }
      ingressFrames += 1;
      ingressBytes += wireLength;
      if (
        ingressFrames > MAX_FRAMES_PER_WINDOW ||
        ingressBytes > MAX_BYTES_PER_WINDOW
      ) {
        throw new Error("ingress_rate_limit");
      }

      const frame = decodeClientFrame(
        toRelayBytesSync(event.data, MAX_MESSAGE_BYTES + 1),
      );
      if (frame.kind === "ack") {
        if (!pendingAck || pendingAck.sequence !== frame.sequence) {
          throw new Error("unexpected_ack");
        }
        pendingAck.resolve();
        pendingAck = undefined;
        lastActivity = Date.now();
        return;
      }

      const pendingLength = frame.payload.byteLength;
      if (
        queuedFrames >= MAX_QUEUED_FRAMES ||
        queuedBytes + pendingLength > MAX_QUEUED_BYTES
      ) {
        throw new Error("queue_limit");
      }
      queuedFrames += 1;
      queuedBytes += pendingLength;
      writeQueue = writeQueue
        .then(async () => {
          try {
            await withTimeout(
              writer.write(frame.payload),
              WRITE_TIMEOUT_MS,
              "tcp_write_timeout",
            );
            lastActivity = Date.now();
          } finally {
            queuedFrames -= 1;
            queuedBytes -= pendingLength;
          }
        })
        .catch(() => finish());
    } catch {
      void finish();
    }
  });
  server.addEventListener("close", () => {
    void finish();
  });
  server.addEventListener("error", () => {
    void finish();
  });

  try {
    while (!closed) {
      const { value, done } = await reader.read();
      if (done) break;
      if (!value) continue;
      lastActivity = Date.now();
      for (let offset = 0; offset < value.byteLength; offset += MAX_MESSAGE_BYTES) {
        const payload = value.slice(offset, offset + MAX_MESSAGE_BYTES);
        const now = Date.now();
        if (now - egressWindowStarted >= RATE_WINDOW_MS) {
          egressWindowStarted = now;
          egressFrames = 0;
          egressBytes = 0;
        }
        egressFrames += 1;
        egressBytes += payload.byteLength;
        if (
          egressFrames > MAX_FRAMES_PER_WINDOW ||
          egressBytes > MAX_BYTES_PER_WINDOW
        ) {
          throw new Error("egress_rate_limit");
        }
        if (pendingAck) {
          throw new Error("ack_state_error");
        }
        const sequence = nextSequence;
        nextSequence = nextSequence === 0xffff_ffff ? 1 : nextSequence + 1;
        const acknowledged = new Promise<void>((resolve, reject) => {
          pendingAck = { sequence, resolve, reject };
        });
        const ackWait = withTimeout(
          acknowledged,
          ACK_TIMEOUT_MS,
          "ack_timeout",
        );
        void ackWait.catch(() => undefined);
        server.send(encodeServerData(sequence, payload));
        await ackWait;
      }
    }
  } finally {
    try {
      reader.releaseLock();
    } catch {
      // Ignore an already released reader.
    }
    await finish();
  }
}
