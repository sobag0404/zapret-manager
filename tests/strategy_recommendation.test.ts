import { describe, expect, it } from "vitest";
import { recommendedEngineStrategy, YOUTUBE_RECOMMENDED_STRATEGY } from "../app/frontend/src/store/appStore";

const disabledYoutubeOnly = {
  selectedProfiles: ["youtube"],
  runtimeStatus: "disabled" as const,
  currentStrategy: "general",
  strategySelectedManually: false,
};

describe("YouTube strategy recommendation", () => {
  it("recommends Fake TLS Auto only for YouTube while disabled with the default strategy", () => {
    expect(recommendedEngineStrategy(disabledYoutubeOnly)).toBe(YOUTUBE_RECOMMENDED_STRATEGY);
  });

  it("does not replace a manual selection, an active engine, or a mixed profile selection", () => {
    expect(recommendedEngineStrategy({ ...disabledYoutubeOnly, strategySelectedManually: true })).toBeNull();
    expect(recommendedEngineStrategy({ ...disabledYoutubeOnly, runtimeStatus: "running" })).toBeNull();
    expect(recommendedEngineStrategy({ ...disabledYoutubeOnly, selectedProfiles: ["youtube", "discord"] })).toBeNull();
  });
});
