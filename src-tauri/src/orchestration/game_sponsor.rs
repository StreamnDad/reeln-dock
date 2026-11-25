use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

const FILENAME: &str = "game-sponsor.png";
const MAX_FILE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_DIMENSION: u32 = 8192;
const MAX_PIXELS: u64 = 16_000_000;
const COLOR_FORMAT_ERROR: &str = "Choose an approved 8-bit sRGB PNG for the sponsor logo.";

pub struct GameSponsorLogo(Vec<u8>);

impl GameSponsorLogo {
    pub fn read(source: &Path) -> Result<Self, String> {
        if !source.is_file() {
            return Err(format!("Sponsor logo file not found: {}", source.display()));
        }
        let file =
            std::fs::File::open(source).map_err(|e| format!("Cannot read sponsor logo: {e}"))?;
        let mut bytes = Vec::new();
        file.take(MAX_FILE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| format!("Cannot read sponsor logo: {e}"))?;
        if bytes.len() as u64 > MAX_FILE_BYTES {
            return Err("Sponsor logo file must be at most 32 MiB.".into());
        }
        let invalid_png = |e| format!("Choose a valid static PNG for the sponsor logo. {e}");

        let mut cursor = Cursor::new(bytes.as_slice());
        let mut decoder = png::Decoder::new(&mut cursor);
        decoder.set_transformations(png::Transformations::EXPAND);
        let header = decoder.read_header_info().map_err(invalid_png)?;
        let (width, height) = header.size();
        if width > MAX_DIMENSION
            || height > MAX_DIMENSION
            || u64::from(width) * u64::from(height) > MAX_PIXELS
        {
            return Err(
                "Sponsor logo is too large to load. Choose a PNG up to 8,192 pixels per side and 16 megapixels."
                    .into(),
            );
        }
        if header.bit_depth == png::BitDepth::Sixteen {
            return Err(COLOR_FORMAT_ERROR.into());
        }

        let mut reader = decoder.read_info().map_err(invalid_png)?;
        if reader.info().animation_control().is_some() {
            return Err("Choose a static PNG for the sponsor logo.".into());
        }
        if !has_compatible_color_metadata(reader.info()) {
            return Err(COLOR_FORMAT_ERROR.into());
        }
        let size = reader
            .output_buffer_size()
            .ok_or("Could not decode sponsor logo pixels.")?;
        let mut pixels = vec![0; size];
        let frame = reader.next_frame(&mut pixels).map_err(invalid_png)?;
        if matches!(
            frame.color_type,
            png::ColorType::Rgba | png::ColorType::GrayscaleAlpha
        ) {
            let channels = frame.color_type.samples();
            if pixels[channels - 1..frame.buffer_size()]
                .iter()
                .step_by(channels)
                .all(|&alpha| alpha == 0)
            {
                return Err("The sponsor logo has no visible pixels. Choose a visible PNG.".into());
            }
        }
        reader.finish().map_err(invalid_png)?;
        drop(reader);
        if cursor.position() != bytes.len() as u64 {
            return Err(
                "Choose a file containing a single static PNG for the sponsor logo.".into(),
            );
        }

        Ok(Self(bytes))
    }

    pub fn save(&self, game_dir: &Path) -> Result<PathBuf, String> {
        let path = game_dir.join(FILENAME);
        let mut temp = tempfile::NamedTempFile::new_in(game_dir)
            .map_err(|e| format!("Cannot prepare sponsor logo: {e}"))?;
        temp.write_all(&self.0)
            .map_err(|e| format!("Cannot write sponsor logo: {e}"))?;
        temp.persist(&path)
            .map_err(|e| format!("Cannot save sponsor logo: {e}"))?;
        Ok(path)
    }

    pub fn find(game_dir: &Path) -> Result<Option<PathBuf>, String> {
        let path = game_dir.join(FILENAME);
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_file() => Ok(Some(path)),
            Ok(_) => Err(format!("Sponsor logo is not a file: {}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("Cannot find sponsor logo: {e}")),
        }
    }

    pub fn remove(game_dir: &Path) -> Result<(), String> {
        match std::fs::remove_file(game_dir.join(FILENAME)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("Cannot remove sponsor logo: {e}")),
        }
    }
}

fn has_compatible_color_metadata(info: &png::Info<'_>) -> bool {
    if info.coding_independent_code_points.is_some() {
        return false;
    }
    // The image compositor checks ICC profile equivalence using LittleCMS.
    if info.icc_profile.is_some() || info.srgb.is_some() {
        return true;
    }
    let matches = |value: png::ScaledFloat, expected| value.into_scaled().abs_diff(expected) <= 1;
    let matches_pair = |(x, y), (want_x, want_y)| matches(x, want_x) && matches(y, want_y);
    info.gama_chunk.is_none_or(|gamma| matches(gamma, 45455))
        && info.chrm_chunk.is_none_or(|chrm| {
            matches_pair(chrm.white, (31270, 32900))
                && matches_pair(chrm.red, (64000, 33000))
                && matches_pair(chrm.green, (30000, 60000))
                && matches_pair(chrm.blue, (15000, 6000))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_png(path: &Path, width: u32, height: u32, animated: bool, value: u8) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut bytes, width, height);
        if animated {
            encoder.set_animated(1, 0).unwrap();
        }
        let mut writer = encoder.write_header().unwrap();
        writer
            .write_image_data(&vec![value; (width * height) as usize])
            .unwrap();
        writer.finish().unwrap();
        std::fs::write(path, &bytes).unwrap();
        bytes
    }

    #[test]
    fn game_sponsor_validates_and_saves_original_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("official.png");
        let original = write_png(&source, 2048, 1024, false, 42);
        assert_eq!(GameSponsorLogo::find(dir.path()).unwrap(), None);

        let asset = GameSponsorLogo::read(&source).unwrap();
        let saved_path = asset.save(dir.path()).unwrap();
        assert_eq!(saved_path, dir.path().join(FILENAME));
        assert_eq!(std::fs::read(&saved_path).unwrap(), original);
        assert_eq!(
            GameSponsorLogo::find(dir.path()).unwrap(),
            Some(saved_path.clone())
        );

        let replacement = write_png(&source, 120, 40, false, 17);
        GameSponsorLogo::read(&source)
            .unwrap()
            .save(dir.path())
            .unwrap();
        assert_eq!(std::fs::read(saved_path).unwrap(), replacement);

        GameSponsorLogo::remove(dir.path()).unwrap();
        GameSponsorLogo::remove(dir.path()).unwrap();
        assert_eq!(GameSponsorLogo::find(dir.path()).unwrap(), None);
        assert_eq!(std::fs::read(source).unwrap(), replacement);
    }

    #[test]
    fn game_sponsor_rejects_oversized_animated_and_corrupt_pngs() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("logo.png");

        write_png(&source, MAX_DIMENSION + 1, 1, false, 0);
        assert!(
            GameSponsorLogo::read(&source)
                .err()
                .unwrap()
                .contains("too large to load")
        );
        write_png(&source, 1, MAX_DIMENSION + 1, false, 0);
        assert!(GameSponsorLogo::read(&source).is_err());
        write_png(&source, 4096, 4096, false, 0);
        assert!(GameSponsorLogo::read(&source).is_err());

        std::fs::File::create(&source)
            .unwrap()
            .set_len(MAX_FILE_BYTES + 1)
            .unwrap();
        assert!(
            GameSponsorLogo::read(&source)
                .err()
                .unwrap()
                .contains("32 MiB")
        );

        write_png(&source, 80, 40, true, 0);
        assert!(
            GameSponsorLogo::read(&source)
                .err()
                .unwrap()
                .contains("static PNG")
        );

        let valid = write_png(&source, 80, 40, false, 0);
        std::fs::write(&source, &valid[..valid.len() - 12]).unwrap();
        assert!(GameSponsorLogo::read(&source).is_err());
        std::fs::write(&source, [valid.as_slice(), valid.as_slice()].concat()).unwrap();
        assert!(GameSponsorLogo::read(&source).is_err());
        std::fs::write(&source, b"Not an image").unwrap();
        assert!(GameSponsorLogo::read(&source).is_err());
    }

    #[test]
    fn game_sponsor_rejects_color_samples_that_cannot_be_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("logo.png");
        for (depth, samples, gamma) in [
            (png::BitDepth::Sixteen, &[0, 0][..], 45455),
            (png::BitDepth::Eight, &[0][..], 60000),
        ] {
            let file = std::fs::File::create(&source).unwrap();
            let mut encoder = png::Encoder::new(file, 1, 1);
            encoder.set_depth(depth);
            encoder.set_source_gamma(png::ScaledFloat::from_scaled(gamma));
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(samples).unwrap();
            writer.finish().unwrap();

            assert_eq!(
                GameSponsorLogo::read(&source).err().as_deref(),
                Some(COLOR_FORMAT_ERROR)
            );
        }
    }

    #[test]
    fn game_sponsor_requires_visible_pixels_in_transparent_artwork() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("logo.png");
        for alpha in [0, 255] {
            let file = std::fs::File::create(&source).unwrap();
            let mut encoder = png::Encoder::new(file, 2, 1);
            encoder.set_color(png::ColorType::Rgba);
            let mut writer = encoder.write_header().unwrap();
            writer
                .write_image_data(&[0, 0, 0, 0, 255, 255, 255, alpha])
                .unwrap();
            writer.finish().unwrap();

            let result = GameSponsorLogo::read(&source);
            if alpha == 0 {
                assert!(result.err().unwrap().contains("no visible pixels"));
            } else {
                assert!(result.is_ok());
            }
        }
    }
}
