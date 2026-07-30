import { describe, expect, it } from "vitest";
import {
  DISCORD_RECOMMENDED_STRATEGY,
  recommendedEngineStrategy,
  YOUTUBE_RECOMMENDED_STRATEGY,
} from "../app/frontend/src/store/appStore";

const disabledYoutubeOnly = {
  selectedProfiles: ["youtube"],
  runtimeStatus: "disabled" as const,
  currentStrategy: "general",
  strategySelectedManually: false,
};

const disabledDiscordOnly = {
  selectedProfiles: ["discord"],
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

describe("Discord strategy recommendation", () => {
  it("recommends 2 ALT only for Discord while disabled with the default strategy", () => {
    expect(recommendedEngineStrategy(disabledDiscordOnly)).toBe(DISCORD_RECOMMENDED_STRATEGY);
  });

  it("does not replace a manual selection, an active engine, or a mixed profile selection", () => {
    expect(recommendedEngineStrategy({ ...disabledDiscordOnly, strategySelectedManually: true })).toBeNull();
    expect(recommendedEngineStrategy({ ...disabledDiscordOnly, runtimeStatus: "running" })).toBeNull();
    expect(recommendedEngineStrategy({ ...disabledDiscordOnly, selectedProfiles: ["discord", "youtube"] })).toBeNull();
    expect(recommendedEngineStrategy({ ...disabledDiscordOnly, currentStrategy: "alt3" })).toBeNull();
  });
});

describe("supported mode recommendation", () => {
  it("does not recommend a strategy for an unsupported or combined selection", () => {
    expect(recommendedEngineStrategy({ ...disabledDiscordOnly, selectedProfiles: ["telegram"] })).toBeNull();
    expect(recommendedEngineStrategy({ ...disabledDiscordOnly, selectedProfiles: ["discord", "youtube"] })).toBeNull();
  });
});
