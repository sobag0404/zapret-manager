export type ConnectionLease = {
  release: () => void;
};

export class ConnectionLimiter {
  private active = 0;

  constructor(private readonly maximum: number) {
    if (!Number.isSafeInteger(maximum) || maximum < 1) {
      throw new Error("invalid_connection_limit");
    }
  }

  acquire(): ConnectionLease | undefined {
    if (this.active >= this.maximum) {
      return undefined;
    }
    this.active += 1;
    let released = false;
    return {
      release: () => {
        if (released) return;
        released = true;
        this.active -= 1;
      },
    };
  }

  activeCount(): number {
    return this.active;
  }
}
