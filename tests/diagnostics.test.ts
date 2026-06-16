import { describe, expect, it } from "vitest";

type Status = "ok" | "warning" | "error" | "skipped";

function aggregate(statuses: Status[]): Status {
  if (statuses.includes("error")) return "error";
  if (statuses.includes("warning")) return "warning";
  if (statuses.every((status) => status === "skipped")) return "skipped";
  return "ok";
}

describe("diagnostics aggregation", () => {
  it("prioritizes errors, then warnings, then skipped", () => {
    expect(aggregate(["ok", "warning"])).toBe("warning");
    expect(aggregate(["ok", "error"])).toBe("error");
    expect(aggregate(["skipped", "skipped"])).toBe("skipped");
    expect(aggregate(["ok", "skipped"])).toBe("ok");
  });
});
