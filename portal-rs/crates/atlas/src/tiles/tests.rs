#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use image::{DynamicImage, Rgba};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "atlas-tiles-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn input(&self) -> PathBuf {
        let path = self.0.join("input");
        fs::create_dir_all(&path).unwrap();
        path
    }
    fn output(&self) -> PathBuf {
        self.0.join("output")
    }
    fn chunk(&self, x: u32, y: u32, floor: u8, image: RgbaImage) -> PathBuf {
        let mut bmp = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(image)
            .write_to(&mut bmp, ImageFormat::Bmp)
            .unwrap();
        let mut compressed = Vec::new();
        lzma_rs::lzma_compress(&mut Cursor::new(bmp.into_inner()), &mut compressed).unwrap();
        let path = self.input().join(format!(
            "minimap-32-{x}-{y}-{floor}-{}.bmp.lzma",
            hash(&compressed)
        ));
        fs::write(&path, compressed).unwrap();
        path
    }
    fn build(&self) -> BuildReport {
        build(
            TilePipeline { enabled: true },
            &self.input(),
            &self.output(),
            "release-a",
        )
        .unwrap()
    }
    fn tile(&self, floor: u8, path: &str) -> RgbaImage {
        image::open(
            self.output()
                .join("release-a")
                .join(floor.to_string())
                .join(path),
        )
        .unwrap()
        .to_rgba8()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn synthetic_lzma_bmp_chunks_produce_tiles_pixels_and_pyramid() {
    let fixture = Fixture::new();
    let mut left = RgbaImage::from_pixel(256, 257, Rgba([255, 0, 0, 255]));
    left.put_pixel(23, 42, Rgba([12, 34, 56, 255]));
    fixture.chunk(1000, 2000, 7, left);
    fixture.chunk(
        1256,
        2000,
        7,
        RgbaImage::from_pixel(1, 257, Rgba([0, 0, 255, 255])),
    );
    fixture.chunk(
        50,
        60,
        8,
        RgbaImage::from_pixel(1, 1, Rgba([0, 255, 0, 255])),
    );
    let report = fixture.build();
    assert_eq!(report.manifests.len(), 2);
    assert_eq!(report.files_written, 8); // Five tiles + manifest on 7; one + manifest on 8.
    let manifest = &report.manifests[0];
    assert_eq!(
        manifest.bounds,
        Bounds {
            min_x: 1000,
            min_y: 2000,
            max_x: 1257,
            max_y: 2257
        }
    );
    assert_eq!(
        manifest.levels,
        vec![
            Level {
                z: 0,
                width: 129,
                height: 129,
                columns: 1,
                rows: 1
            },
            Level {
                z: 1,
                width: 257,
                height: 257,
                columns: 2,
                rows: 2
            },
        ]
    );
    assert_eq!(manifest.tiles.len(), 5);
    let tile = fixture.tile(7, "1/0/0.webp");
    assert_eq!(tile.dimensions(), (256, 256));
    assert_eq!(*tile.get_pixel(23, 42), Rgba([12, 34, 56, 255]));
    let edge = fixture.tile(7, "1/1/1.webp");
    assert_eq!(*edge.get_pixel(0, 0), Rgba([0, 0, 255, 255]));
    assert_eq!(*edge.get_pixel(1, 0), Rgba([0, 0, 0, 0]));
    assert_eq!(
        *fixture.tile(7, "0/0/0.webp").get_pixel(10, 10),
        Rgba([255, 0, 0, 255])
    );
    assert_eq!(
        *fixture.tile(8, "0/0/0.webp").get_pixel(0, 0),
        Rgba([0, 255, 0, 255])
    );
    let json: Manifest = serde_json::from_slice(
        &fs::read(fixture.output().join("release-a/7/manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(&json, manifest);
}

#[test]
fn repeated_build_skips_unchanged_files_and_repairs_corruption() {
    let fixture = Fixture::new();
    fixture.chunk(
        0,
        0,
        0,
        RgbaImage::from_pixel(513, 3, Rgba([23, 45, 67, 255])),
    );
    let first = fixture.build();
    assert_eq!(
        first.manifests[0]
            .levels
            .iter()
            .map(|l| (l.width, l.height))
            .collect::<Vec<_>>(),
        vec![(129, 1), (257, 2), (513, 3)]
    );
    assert_eq!(first.manifests[0].tiles.len(), 6);
    let path = fixture.output().join("release-a/0/2/0/0.webp");
    let before = fs::metadata(&path).unwrap().modified().unwrap();
    let second = fixture.build();
    assert_eq!(first.manifests, second.manifests);
    assert_eq!(second.files_written, 0);
    assert_eq!(second.files_unchanged, 7);
    assert_eq!(before, fs::metadata(&path).unwrap().modified().unwrap());
    let original = fs::read(&path).unwrap();
    fs::write(&path, b"broken").unwrap();
    let repaired = fixture.build();
    assert_eq!(repaired.files_written, 1);
    assert_eq!(repaired.files_unchanged, 6);
    assert_eq!(original, fs::read(&path).unwrap());
    let other = fixture.0.join("other");
    let independent = build(
        TilePipeline { enabled: true },
        &fixture.input(),
        &other,
        "release-a",
    )
    .unwrap();
    assert_eq!(first.manifests, independent.manifests);
    for tile in &first.manifests[0].tiles {
        assert_eq!(
            fs::read(fixture.output().join("release-a/0").join(&tile.path)).unwrap(),
            fs::read(other.join("release-a/0").join(&tile.path)).unwrap()
        );
    }
}

#[test]
fn default_flag_refuses_before_filesystem_access() {
    let fixture = Fixture::new();
    let error = build(
        TilePipeline::default(),
        &fixture.0.join("absent"),
        &fixture.output(),
        "release-a",
    )
    .unwrap_err();
    assert_eq!(
        error.downcast_ref::<io::Error>().unwrap().kind(),
        io::ErrorKind::PermissionDenied
    );
    assert!(!fixture.output().exists());
}

#[test]
fn bad_names_versions_and_empty_input_are_errors() {
    let fixture = Fixture::new();
    for version in ["", "..", "../escape", "/absolute", "a/b", "a.b"] {
        assert!(
            build(
                TilePipeline { enabled: true },
                &fixture.input(),
                &fixture.output(),
                version
            )
            .is_err()
        );
    }
    assert!(
        build(
            TilePipeline { enabled: true },
            &fixture.input(),
            &fixture.output(),
            "release-a"
        )
        .is_err()
    );
    for name in [
        "minimap-32-1-2-7-bad.bmp.lzma",
        "minimap-32-x-2-7-0123456789012345678901234567890123456789.bmp.lzma",
        "minimap-32-1-2-999-0123456789012345678901234567890123456789.bmp.lzma",
    ] {
        assert!(parse_name(name).is_err());
    }
    fs::write(fixture.input().join("minimap-bad.bmp.lzma"), b"invalid").unwrap();
    assert!(
        build(
            TilePipeline { enabled: true },
            &fixture.input(),
            &fixture.output(),
            "release-a"
        )
        .is_err()
    );
    assert!(!fixture.output().exists());
}

#[test]
fn corrupted_lzma_and_bmp_return_errors_without_output() {
    for payload in [vec![], vec![0; 20], vec![255; 80], b"not lzma".to_vec()] {
        let fixture = Fixture::new();
        let path = fixture.chunk(0, 0, 7, RgbaImage::new(2, 2));
        fs::write(path, payload).unwrap();
        assert!(
            build(
                TilePipeline { enabled: true },
                &fixture.input(),
                &fixture.output(),
                "release-a"
            )
            .is_err()
        );
        assert!(!fixture.output().exists());
    }
    let fixture = Fixture::new();
    let path = fixture.chunk(0, 0, 7, RgbaImage::new(2, 2));
    let mut compressed = Vec::new();
    lzma_rs::lzma_compress(&mut Cursor::new(b"not a bitmap"), &mut compressed).unwrap();
    fs::write(path, compressed).unwrap();
    assert!(
        build(
            TilePipeline { enabled: true },
            &fixture.input(),
            &fixture.output(),
            "release-a"
        )
        .is_err()
    );
    assert!(!fixture.output().exists());
}

#[test]
fn overlap_coordinate_overflow_and_excessive_bounds_are_refused() {
    for (first_x, second_x) in [(0, 1), (u32::MAX, 0), (0, 100_000_000)] {
        let fixture = Fixture::new();
        fixture.chunk(
            first_x,
            0,
            7,
            RgbaImage::from_pixel(2, 2, Rgba([1, 2, 3, 255])),
        );
        fixture.chunk(
            second_x,
            0,
            7,
            RgbaImage::from_pixel(2, 2, Rgba([4, 5, 6, 255])),
        );
        assert!(
            build(
                TilePipeline { enabled: true },
                &fixture.input(),
                &fixture.output(),
                "release-a"
            )
            .is_err()
        );
        assert!(!fixture.output().exists());
    }
}

#[test]
fn sparse_chunks_preserve_native_origin_and_transparent_gaps() {
    let fixture = Fixture::new();
    fixture.chunk(10, 20, 3, RgbaImage::from_pixel(1, 1, Rgba([1, 2, 3, 255])));
    fixture.chunk(12, 20, 3, RgbaImage::from_pixel(1, 1, Rgba([4, 5, 6, 255])));
    let report = fixture.build();
    assert_eq!(
        report.manifests[0].bounds,
        Bounds {
            min_x: 10,
            min_y: 20,
            max_x: 13,
            max_y: 21
        }
    );
    let tile = fixture.tile(3, "0/0/0.webp");
    assert_eq!(*tile.get_pixel(0, 0), Rgba([1, 2, 3, 255]));
    assert_eq!(*tile.get_pixel(1, 0), Rgba([0, 0, 0, 0]));
    assert_eq!(*tile.get_pixel(2, 0), Rgba([4, 5, 6, 255]));
}
