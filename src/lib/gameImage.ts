interface GameImageResult {
  image_path?: string;
  team_logos_composited?: boolean;
  sponsor_logo_applied?: boolean;
  existing_artwork_preserved?: boolean;
}

export function getCompositedGameImage(
  shared: Record<string, unknown>,
  hasSponsor: boolean,
): string | null {
  const image = shared.game_image as GameImageResult | undefined;
  if (typeof image?.image_path !== "string" || !image.image_path) return null;
  if (image.team_logos_composited !== true) {
    throw new Error("Update the OpenAI plugin to 0.12.0 or later to generate a game image with the original logos.");
  }
  if (image.sponsor_logo_applied !== hasSponsor) {
    throw new Error("The generated image does not match this game's sponsor selection. Regenerate the image.");
  }
  return image.image_path;
}

export function getSponsoredExistingGameImage(shared: Record<string, unknown>): string | null {
  const image = shared.game_image as GameImageResult | undefined;
  if (typeof image?.image_path !== "string" || !image.image_path) return null;
  if (image.existing_artwork_preserved !== true || image.sponsor_logo_applied !== true) {
    throw new Error("The image result did not confirm that the original artwork and sponsor were included.");
  }
  return image.image_path;
}
