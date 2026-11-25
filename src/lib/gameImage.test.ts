import { describe, expect, it } from "vitest";
import { getCompositedGameImage, getSponsoredExistingGameImage } from "./gameImage";

describe("getCompositedGameImage", () => {
  const imagePath = "/images/game-image.png";

  it("ignores missing images and rejects images from a renderer that can redraw logos", () => {
    expect(getCompositedGameImage({}, false)).toBeNull();
    expect(() => getCompositedGameImage({ game_image: { image_path: imagePath } }, true))
      .toThrow("Update the OpenAI plugin");
  });

  it.each([true, false])("accepts composited logos when sponsor selection is %s", (hasSponsor) => {
    const shared = {
      game_image: {
        image_path: imagePath,
        team_logos_composited: true,
        sponsor_logo_applied: hasSponsor,
      },
    };
    expect(getCompositedGameImage(shared, hasSponsor)).toBe(imagePath);
    expect(() => getCompositedGameImage(shared, !hasSponsor)).toThrow("sponsor selection");
  });
});

describe("getSponsoredExistingGameImage", () => {
  it("requires the existing artwork and sponsor in the rendered result", () => {
    const imagePath = "/images/sponsored-game-image.png";
    expect(getSponsoredExistingGameImage({})).toBeNull();
    expect(() => getSponsoredExistingGameImage({ game_image: { image_path: imagePath, sponsor_logo_applied: true } }))
      .toThrow("original artwork and sponsor");
    expect(getSponsoredExistingGameImage({
      game_image: { image_path: imagePath, sponsor_logo_applied: true, existing_artwork_preserved: true },
    })).toBe(imagePath);
  });
});
