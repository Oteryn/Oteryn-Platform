// SOURCE: Oteryn/Oteryn-Game @ 857f15f376907c592bb28d854fccff18a15cdf87,
// crates/client-assets/src/{appearance,catalog,sheets}.rs. Synthetic data only.
use super::*;
use std::io::Cursor;
const VERSION: &str = "857f15f376907c592bb28d854fccff18a15cdf87";
fn request(kind: Kind, id: u32) -> Request {
    Request {
        kind,
        appearance_id: id,
        pattern: None,
    }
}
fn appearance(id: u32, info: proto::Info) -> proto::Appearance {
    proto::Appearance {
        id: Some(id),
        groups: vec![proto::Group { info: Some(info) }],
    }
}
fn info(ids: Vec<u32>) -> proto::Info {
    proto::Info {
        sprites: ids,
        ..Default::default()
    }
}

fn sheet(kind: u32, top_down: bool) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let (w, h) = sheets::geometry(kind)?;
    let mut bmp = vec![0_u8; 54 + 384 * 384 * 4];
    bmp[0..2].copy_from_slice(b"BM");
    let len = bmp.len() as u32;
    for (at, value) in [
        (2, len),
        (10, 54),
        (14, 40),
        (18, 384),
        (22, if top_down { (-384_i32) as u32 } else { 384 }),
    ] {
        bmp[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    bmp[26] = 1;
    bmp[28] = 32;
    for y in 0..384_u32 {
        for x in 0..384_u32 {
            let slot = y / h * (384 / w) + x / w;
            let color = match slot {
                0 => [200, 10, 20, 255],
                1 => [10, 200, 20, 255],
                2 => [10, 20, 200, 255],
                3 => [100, 200, 50, 128],
                _ => [255, 0, 255, 255],
            };
            // Mark the upper left pixel of each sprite for orientation checks.
            let [r, g, b, a] = if x % w == 0 && y % h == 0 {
                [1, 2, 3, 255]
            } else {
                color
            };
            let row = if top_down { y } else { 383 - y };
            let at = 54 + ((row * 384 + x) * 4) as usize;
            bmp[at..at + 4].copy_from_slice(&[b, g, r, a]);
        }
    }
    let mut compressed = vec![0; 32];
    lzma_rs::lzma_compress(&mut Cursor::new(bmp), &mut compressed)?;
    // Game assets may store an unreliable uncompressed-size field.
    compressed[37..45].fill(0);
    Ok(compressed)
}
fn fixture(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let entries = serde_json::json!([
        {"type":"appearances","file":"appearances-test.dat"},
        {"type":"sprite","file":"sprites-test.bmp.lzma","firstspriteid":100,"lastspriteid":135,"spritetype":3}
    ]);
    fs::write(
        root.join("catalog-content.json"),
        serde_json::to_vec(&entries)?,
    )?;
    let appearances = proto::Appearances {
        objects: vec![
            appearance(10, info(vec![100])),
            appearance(
                11,
                proto::Info {
                    layers: Some(2),
                    ..info(vec![100, 103])
                },
            ),
            appearance(
                12,
                proto::Info {
                    width: Some(2),
                    animation: Some(proto::Animation {
                        phases: vec![proto::Phase {}, proto::Phase {}],
                    }),
                    ..info(vec![100, 101, 102, 103])
                },
            ),
            appearance(13, info(vec![0])),
        ],
        outfits: vec![appearance(
            20,
            proto::Info {
                width: Some(4),
                layers: Some(2),
                ..info(vec![100, 101, 100, 101, 102, 103, 100, 101])
            },
        )],
    };
    fs::write(
        root.join("appearances-test.dat"),
        appearances.encode_to_vec(),
    )?;
    fs::write(root.join("sprites-test.bmp.lzma"), sheet(3, false)?)?;
    Ok(())
}
fn run(root: &Path, out: &Path, requests: &[Request]) -> Result<Generated, Error> {
    generate(
        SpriteFeature { enabled: true },
        VERSION,
        root,
        out,
        requests,
    )
}
fn image(result: &Generated, at: usize) -> Result<RgbaImage, Box<dyn std::error::Error>> {
    Ok(image::load_from_memory_with_format(
        &fs::read(result.directory.join(&result.manifest.thumbnails[at].file))?,
        image::ImageFormat::WebP,
    )?
    .into_rgba8())
}
#[test]
fn known_items_creatures_first_phase_and_alpha_composition()
-> Result<(), Box<dyn std::error::Error>> {
    let input = tempfile::tempdir()?;
    let output = tempfile::tempdir()?;
    fixture(input.path())?;
    let result = run(
        input.path(),
        output.path(),
        &[
            request(Kind::Item, 10),
            request(Kind::Item, 11),
            request(Kind::Item, 12),
            request(Kind::Item, 13),
            request(Kind::Creature, 20),
        ],
    )?;
    let item = image(&result, 0)?;
    assert_eq!(item.dimensions(), (64, 64));
    assert_eq!(item.get_pixel(0, 0).0, [1, 2, 3, 255]);
    assert_eq!(item.get_pixel(1, 1).0, [200, 10, 20, 255]);
    let composed = image(&result, 1)?.get_pixel(1, 1).0;
    assert_eq!(composed[3], 255);
    assert!((149..=150).contains(&composed[0]));
    assert!((104..=105).contains(&composed[1]));
    assert!((34..=35).contains(&composed[2]));
    assert_eq!(image(&result, 2)?.get_pixel(1, 1).0, [200, 10, 20, 255]);
    assert_eq!(image(&result, 3)?.dimensions(), (32, 32));
    assert!(image(&result, 3)?.pixels().all(|p| p.0 == [0; 4]));
    assert_eq!(image(&result, 4)?.get_pixel(1, 1).0, [10, 20, 200, 255]);
    assert_eq!(result.manifest.thumbnails[4].pattern, [2, 0, 0]);
    Ok(())
}
#[test]
fn all_layouts_row_order_slots_and_magenta_are_correct() -> Result<(), Box<dyn std::error::Error>> {
    for kind in 0..=3 {
        for top_down in [false, true] {
            let decoded = sheets::decode(&sheet(kind, top_down)?)?;
            let (w, h) = sheets::geometry(kind)?;
            assert_eq!(decoded.dimensions(), (384, 384));
            assert_eq!(decoded.get_pixel(0, 0).0, [1, 2, 3, 255]);
            assert_eq!(decoded.get_pixel(w + 1, 1).0, [10, 200, 20, 255]);
            assert_eq!(decoded.get_pixel(1, h - 1).0, [200, 10, 20, 255]);
            assert_eq!(decoded.get_pixel(383, 383).0, [0; 4]);
        }
    }
    Ok(())
}
#[test]
fn deterministic_reordered_requests_reuse_and_corruption_refusal()
-> Result<(), Box<dyn std::error::Error>> {
    let input = tempfile::tempdir()?;
    let output = tempfile::tempdir()?;
    fixture(input.path())?;
    let requests = [request(Kind::Creature, 20), request(Kind::Item, 10)];
    let first = run(input.path(), output.path(), &requests)?;
    assert!(!first.reused);
    let path = first.directory.join(&first.manifest.thumbnails[0].file);
    let modified = fs::metadata(&path)?.modified()?;
    let second = run(input.path(), output.path(), &[requests[1], requests[0]])?;
    assert!(second.reused);
    assert_eq!(first.manifest, second.manifest);
    assert_eq!(first.directory, second.directory);
    assert_eq!(fs::metadata(&path)?.modified()?, modified);
    let other_output = tempfile::tempdir()?;
    let third = run(input.path(), other_output.path(), &requests)?;
    assert_eq!(third.manifest, first.manifest);
    fs::write(&path, b"corrupt")?;
    assert!(run(input.path(), output.path(), &requests).is_err());
    Ok(())
}
#[test]
fn feature_off_and_invalid_version_refuse_before_io() -> Result<(), Box<dyn std::error::Error>> {
    let absent = Path::new("/no-such-release");
    assert!(matches!(
        generate(SpriteFeature::default(), VERSION, absent, absent, &[]),
        Err(Error::Disabled)
    ));
    assert!(matches!(
        generate(
            SpriteFeature { enabled: true },
            "../escape",
            absent,
            absent,
            &[]
        ),
        Err(Error::Invalid(_))
    ));
    Ok(())
}
#[test]
fn bad_requests_never_publish_a_partial_batch() -> Result<(), Box<dyn std::error::Error>> {
    let input = tempfile::tempdir()?;
    let output = tempfile::tempdir()?;
    fixture(input.path())?;
    let item = request(Kind::Item, 10);
    for requests in [
        vec![],
        vec![item, item],
        vec![request(Kind::Item, 999)],
        vec![Request {
            pattern: Some([99, 0, 0]),
            ..item
        }],
    ] {
        assert!(run(input.path(), output.path(), &requests).is_err());
    }
    let parent = output.path().join(VERSION);
    if parent.exists() {
        assert_eq!(fs::read_dir(parent)?.count(), 0);
    }
    Ok(())
}
#[test]
fn corrupt_inputs_are_errors() -> Result<(), Box<dyn std::error::Error>> {
    for bytes in [vec![], vec![0; 46], vec![255; 100]] {
        assert!(sheets::decode(&bytes).is_err());
    }
    let input = tempfile::tempdir()?;
    let output = tempfile::tempdir()?;
    fixture(input.path())?;
    fs::write(input.path().join("appearances-test.dat"), [0xff])?;
    assert!(run(input.path(), output.path(), &[request(Kind::Item, 10)]).is_err());
    fixture(input.path())?;
    fs::write(input.path().join("sprites-test.bmp.lzma"), [0; 100])?;
    assert!(run(input.path(), output.path(), &[request(Kind::Item, 10)]).is_err());
    assert_eq!(fs::read_dir(output.path().join(VERSION))?.count(), 0);
    Ok(())
}
#[test]
fn catalog_and_reference_validation() -> Result<(), Box<dyn std::error::Error>> {
    for bytes in [
        br#"[{"type":"appearances","file":"../bad.dat"}]"#.as_slice(),
        br#"[{"type":"appearances","file":"ok.dat"},{"type":"sprite","file":"ok.bmp.lzma","firstspriteid":1,"lastspriteid":99999,"spritetype":0}]"#,
        br#"[]"#,
    ] { assert!(catalog(bytes).is_err()); }
    assert!(info(vec![]).dimensions().is_err());
    assert!(
        proto::Info {
            width: Some(0),
            ..info(vec![1])
        }
        .dimensions()
        .is_err()
    );
    assert!(
        index(vec![
            appearance(1, info(vec![1])),
            appearance(1, info(vec![1]))
        ])
        .is_err()
    );
    Ok(())
}
#[test]
fn explicit_pattern_selects_correct_variant() -> Result<(), Box<dyn std::error::Error>> {
    let input = tempfile::tempdir()?;
    let output = tempfile::tempdir()?;
    fixture(input.path())?;
    let result = run(
        input.path(),
        output.path(),
        &[Request {
            pattern: Some([1, 0, 0]),
            ..request(Kind::Item, 12)
        }],
    )?;
    assert_eq!(image(&result, 0)?.get_pixel(1, 1).0, [10, 200, 20, 255]);
    assert_eq!(result.manifest.sources.len(), 3);
    let old = result.directory;
    let mut bytes = fs::read(input.path().join("appearances-test.dat"))?;
    // Unknown protobuf field, preserving pixels while changing source identity.
    bytes.extend_from_slice(&[0x78, 1]);
    fs::write(input.path().join("appearances-test.dat"), bytes)?;
    let changed = run(
        input.path(),
        output.path(),
        &[Request {
            pattern: Some([1, 0, 0]),
            ..request(Kind::Item, 12)
        }],
    )?;
    assert_ne!(old, changed.directory);
    Ok(())
}

#[test]
fn pinned_release_compatibility_when_available() -> Result<(), Box<dyn std::error::Error>> {
    let Ok(root) = std::env::var("PORTAL_SPRITE_ASSETS_DIR") else {
        eprintln!("SKIPPED real asset smoke test: PORTAL_SPRITE_ASSETS_DIR is unset");
        return Ok(());
    };
    let output = tempfile::tempdir()?;
    let result = run(
        Path::new(&root),
        output.path(),
        &[request(Kind::Item, 100), request(Kind::Creature, 2)],
    )?;
    assert_eq!(result.manifest.thumbnails.len(), 2);
    for entry in &result.manifest.thumbnails {
        assert!([32, 64].contains(&entry.width));
        assert!([32, 64].contains(&entry.height));
    }
    assert!(image(&result, 0)?.pixels().any(|p| p[3] > 0));
    assert!(image(&result, 1)?.pixels().any(|p| p[3] > 0));
    assert!(
        run(
            Path::new(&root),
            output.path(),
            &[request(Kind::Item, 100), request(Kind::Creature, 2)]
        )?
        .reused
    );
    Ok(())
}
