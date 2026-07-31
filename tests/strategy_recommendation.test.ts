import { describe, expect, it } from "vitest";
import {
  DISCORD_RECOMMENDED_STRATEGY,
  COMBINED_RECOMMENDED_STRATEGY,
  nextSelectedProfiles,
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

  it("does not replace a manual selection or an active engine", () => {
    expect(recommendedEngineStrategy({ ...disabledYoutubeOnly, strategySelectedManually: true })).toBeNull();
    expect(recommendedEngineStrategy({ ...disabledYoutubeOnly, runtimeStatus: "running" })).toBeNull();
  });
});

describe("Discord strategy recommendation", () => {
  it("recommends 2 ALT only for Discord while disabled with the default strategy", () => {
    expect(recommendedEngineStrategy(disabledDiscordOnly)).toBe(DISCORD_RECOMMENDED_STRATEGY);
  });

  it("does not replace a manual selection, an active engine, or an unrelated strategy", () => {
    expect(recommendedEngineStrategy({ ...disabledDiscordOnly, strategySelectedManually: true })).toBeNull();
    expect(recommendedEngineStrategy({ ...disabledDiscordOnly, runtimeStatus: "running" })).toBeNull();
    expect(recommendedEngineStrategy({ ...disabledDiscordOnly, currentStrategy: "alt3" })).toBeNull();
  });
});

describe("supported mode recommendation", () => {
  it("recommends the composed strategy for Discord and YouTube together", () => {
    expect(recommendedEngineStrategy({ ...disabledDiscordOnly, selectedProfiles: ["discord", "youtube"], currentStrategy: "alt" }))
      .toBe(COMBINED_RECOMMENDED_STRATEGY);
  });

  it("does not recommend a strategy for an unsupported selection", () => {
    expect(recommendedEngineStrategy({ ...disabledDiscordOnly, selectedProfiles: ["telegram"] })).toBeNull();
  });

  it("keeps both supported checkboxes selected and preserves canonical order", () => {
    expect(nextSelectedProfiles([], "youtube", true)).toEqual(["youtube"]);
    expect(nextSelectedProfiles(["youtube"], "discord", true)).toEqual(["discord", "youtube"]);
    expect(nextSelectedProfiles(["discord", "youtube"], "discord", false)).toEqual(["youtube"]);
    expect(nextSelectedProfiles(["discord"], "telegram", true)).toEqual(["discord"]);
  });
});
