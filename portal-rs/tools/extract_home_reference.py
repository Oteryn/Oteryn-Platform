#!/usr/bin/env python3
"""Extract lossless source pixels from the owner's 1672x941 portal reference.

No painting, resampling, generated art, or reconstructed background pixels.
Masks only remove source pixels. Every retained RGBA pixel and every decoded
lossless WebP is checked against its exact source crop.
"""

import argparse
import hashlib
import json
from pathlib import Path

from PIL import Image, __version__ as pillow_version, features

HERE = Path(__file__).resolve().parent
REFERENCE_SIZE = (1672, 941)
REFERENCE_SHA256 = "08090697dd979efca7fa0afd2b4e97751af1b4723245a9aca904df7cd277725a"

# Boxes are half-open source coordinates [left, top, right, bottom].
CROPS = {
    "top-scene": ([0, 0, 1672, 75], "Top scenery, original logo and banner tops"),
    "nav-band": ([0, 75, 1672, 125], "Original eight Polish navigation labels and active Home artwork"),
    "nav-neutral": ([1000, 84, 1034, 111], "Clean navigation fill without text or icon"),
    "nav-ornament": ([801, 113, 869, 129], "Center ornament; retain its intrinsic dimensions"),
    "nav-bottom-edge": ([900, 113, 1000, 121], "Clean original bottom edge without labels or center ornament; responsive navigation strip"),
    "logo-top": ([694, 0, 981, 77], "Original logo with its natural scenery"),
    "banner-left": ([133, 0, 206, 166], "Left banner with its natural scenery"),
    "banner-right": ([1466, 0, 1540, 166], "Right banner with its natural scenery"),
    "hero-left": ([0, 125, 598, 350], "Left hero art and source background, full stage edge"),
    "hero-center": ([598, 125, 1116, 350], "Original static Polish hero copy and both CTA buttons"),
    "hero-copy": ([598, 125, 1116, 274], "Original static Polish hero title and lead, without buttons"),
    "hero-title": ([635, 137, 1048, 180], "Original Polish hero title and underlying scenery"),
    "hero-lead": ([645, 186, 1030, 259], "Original Polish hero lead and underlying scenery"),
    "hero-texture": ([1055, 181, 1109, 250], "Clean source forest/mountain patch beside the copy; no lettering"),
    "hero-right": ([1388, 125, 1672, 350], "Monster, source scenery and right stage edge"),
    "heroes": ([62, 122, 592, 348], "Original-size art alternative for responsive adaptation"),
    "monster": ([1388, 122, 1622, 348], "Original-size art alternative for responsive adaptation"),
    "button-register": ([598, 275, 825, 330], "Original static Polish red CTA, icon, lettering and glow"),
    "button-download": ([834, 275, 1077, 330], "Original static Polish green CTA, icon, lettering and glow"),
    "button-register-fill": ([623, 291, 637, 313], "Clean red button fill without lettering or icon"),
    "button-download-fill": ([1058, 290, 1064, 314], "Clean green button fill without lettering or icon"),
    "status-texture": ([1240, 204, 1255, 220], "Clean source status texture; no reference values"),
    "status-heading": ([1203, 143, 1368, 167], "Original static Polish status heading; green LED excluded"),
    "status-led": ([1176, 141, 1204, 168], "Original status LED; use only when the actual state warrants the active indicator"),
    "status-icon-online": ([1140, 176, 1164, 198], "Original online icon; no row label or value"),
    "status-icon-server-save": ([1140, 203, 1164, 225], "Original coin/server-save icon; no row label or value"),
    "status-icon-exp": ([1140, 229, 1164, 251], "Original experience icon; no row label or value"),
    "status-icon-loot": ([1140, 255, 1164, 277], "Original chest/loot icon; no row label or value"),
    "status-icon-pvp": ([1140, 282, 1164, 304], "Original swords icon; no row label or value"),
    "ranking-texture": ([1254, 528, 1354, 549], "Clean source ranking texture; no reference values"),
    "paper-texture": ([332, 473, 780, 495], "Clean parchment between heading and all-news link"),
    "body-texture": ([724, 448, 944, 458], "Clean stone strip between features and content"),
    "news-heading": ([158, 464, 330, 502], "Original static Polish news heading and icon"),
    "news-picture-1": ([164, 511, 428, 582], "Original news art and static Aktualizacja badge; no story title/date"),
    "news-picture-2": ([445, 511, 707, 582], "Original news art and static Wydarzenie badge; no story title/date"),
    "news-picture-3": ([722, 511, 981, 582], "Original news art and static Zmiany badge; no story title/date"),
    "news-badge-fill": ([168, 516, 173, 530], "Clean red category-badge fill; no source lettering"),
    "calendar-icon": ([176, 588, 190, 603], "Original date calendar icon with natural parchment; no date value"),
    "medal-1": ([1050, 522, 1094, 552], "Original first-place medal at intrinsic 44x30 size"),
    "medal-2": ([1050, 553, 1094, 583], "Original second-place medal at intrinsic 44x30 size"),
    "medal-3": ([1050, 584, 1094, 614], "Original third-place medal at intrinsic 44x30 size"),
    "avatar-1": ([1122, 522, 1150, 552], "Original decorative avatar at intrinsic 28x30; no player name or level"),
    "avatar-2": ([1122, 553, 1150, 583], "Original decorative avatar at intrinsic 28x30; no player name or level"),
    "avatar-3": ([1122, 584, 1150, 614], "Original decorative avatar at intrinsic 28x30; no player name or level"),
    "avatar-4": ([1122, 615, 1150, 645], "Original decorative avatar at intrinsic 28x30; no player name or level"),
    "avatar-5": ([1122, 647, 1150, 677], "Original decorative avatar at intrinsic 28x30; no player name or level"),
    "ranking-heading": ([1038, 466, 1260, 499], "Original static Polish ranking heading and icon"),
    "gallery-panel": ([126, 716, 1547, 858], "Original static gallery, heading and community CTA"),
    "gallery-band": ([0, 716, 1672, 860], "Original static gallery and full stage edges"),
    "button-community": ([1319, 719, 1528, 751], "Original static Polish gallery CTA"),
    "footer-band": ([0, 860, 1672, 941], "Original static footer; its source copyright year is 2025"),
    "footer-logo": ([196, 868, 356, 934], "Original footer logo at intrinsic 160x66 size"),
    "footer-texture": ([390, 920, 1255, 940], "Clean footer texture below text"),
    "side-left": ([0, 0, 128, 941], "Original full left architectural surround; do not repeat vertically"),
    "side-right": ([1544, 0, 1672, 941], "Original full right architectural surround; do not repeat vertically"),
}

FEATURE_BOXES = [
    [126, 347, 418, 447], [427, 347, 698, 447], [708, 347, 977, 447],
    [986, 347, 1259, 447], [1268, 347, 1547, 447],
]
GALLERY_BOXES = [
    [156, 757, 431, 844], [444, 757, 703, 844], [716, 757, 975, 844],
    [988, 757, 1247, 844], [1261, 757, 1523, 844],
]
NAV_ICON_BOXES = {
    "home": [298, 84, 324, 110], "news": [468, 84, 494, 110],
    "download": [616, 84, 642, 110], "register": [738, 84, 768, 110],
    "ranking": [900, 84, 928, 110], "wiki": [1038, 84, 1068, 110],
    "community": [1146, 84, 1174, 110], "shop": [1306, 84, 1336, 110],
}
for index, box in enumerate(FEATURE_BOXES, 1):
    CROPS[f"feature-{index}"] = (box, "Original static Polish feature card, with unchanged source copy")
for index, box in enumerate(GALLERY_BOXES, 1):
    CROPS[f"gallery-{index}"] = (box, "Original gallery illustration, border and intrinsic dimensions")
for name, box in NAV_ICON_BOXES.items():
    CROPS[f"nav-icon-{name}"] = (box, "Original-size navigation icon and its natural background")

# Full source bands with the *entire* dynamic panels removed, including borders.
# The removed rectangles contain no hidden RGB source data in the output.
MASKED_CROPS = {
    "hero-scene": ([0, 125, 1672, 350], [[1116, 133, 1387, 326]],
                   "Hero artwork and static copy; dynamic status panel entirely removed"),
    "hero-status-surround": ([1116, 125, 1388, 350], [[1116, 133, 1387, 326]],
                             "Original scenery above/below status; no panel data"),
    "hero-center-surround": ([598, 125, 1116, 350],
                             [[635, 137, 1048, 180], [645, 186, 1030, 259],
                              [598, 275, 825, 330], [834, 275, 1077, 330]],
                             "Original scenery with title, lead and both buttons removed for native EN/auth variants"),
    "features-surround": ([0, 347, 1672, 458], FEATURE_BOXES,
                          "Full stage edge and gaps; overlay the five original feature cards"),
    "content-surround": ([0, 458, 1672, 716], [[125, 458, 1007, 710], [1016, 460, 1455, 701]],
                         "Architecture, torches and hanging banner; news/ranking entirely removed"),
    "news-picture-1-without-badge": ([164, 511, 428, 582], [[164, 511, 253, 536]],
                                     "Original art; entire static category badge removed for a native category overlay"),
    "news-picture-2-without-badge": ([445, 511, 707, 582], [[445, 511, 536, 536]],
                                     "Original art; entire static category badge removed for a native category overlay"),
    "news-picture-3-without-badge": ([722, 511, 981, 582], [[722, 511, 791, 536]],
                                     "Original art; entire static category badge removed for a native category overlay"),
    "news-surface": ([125, 458, 1007, 710],
                     [[163, 510, 430, 694], [443, 510, 710, 694], [722, 510, 984, 694],
                      [158, 464, 330, 502], [785, 469, 981, 496]],
                     "Exact parchment and panel frame; all cards, heading and all-news label removed for existing native elements"),
    "news-card-surface-1": ([163, 510, 430, 694],
                            [[164, 511, 253, 535], [192, 588, 262, 605], [174, 604, 423, 688]],
                            "Exact first card art/frame/parchment; category/date/title/excerpt/read-more removed; render only for an actual news entry"),
    "news-card-surface-2": ([443, 510, 710, 694],
                            [[445, 511, 534, 535], [472, 588, 542, 605], [454, 604, 703, 688]],
                            "Exact second card art/frame/parchment; category/date/title/excerpt/read-more removed; render only for an actual news entry"),
    "news-card-surface-3": ([722, 510, 984, 694],
                            [[722, 511, 786, 535], [752, 588, 822, 605], [734, 604, 980, 688]],
                            "Exact third card art/frame/parchment; category/date/title/excerpt/read-more removed; render only for an actual news entry"),
    "ranking-surface": ([1016, 460, 1455, 701],
                        [[1042, 522, 1429, 678], [1038, 466, 1260, 499], [1337, 471, 1437, 498],
                         [1065, 504, 1082, 521], [1118, 504, 1155, 521], [1357, 504, 1402, 521]],
                        "Exact ranking frame/header texture; full tbody, heading, all-ranking link and column captions removed for native elements"),
    "status-live-surface": ([1116, 133, 1387, 326],
                            [[1176, 141, 1204, 168], [1332, 177, 1366, 199], [1254, 203, 1366, 226],
                             [1331, 230, 1366, 253], [1331, 257, 1366, 280], [1297, 284, 1366, 307]],
                            "Exact Polish status panel/static labels; LED and every sample value removed; use only when an actual status list exists, never as unavailable/English fallback"),
}

RANKING_ROW_BOXES = [
    [1042, 522, 1429, 553], [1042, 553, 1429, 584], [1042, 584, 1429, 615],
    [1042, 615, 1429, 646], [1042, 646, 1429, 678],
]
# Tight source lettering bounds plus a 3px margin; preserve the surrounding row texture.
RANKING_NAME_BOXES = [
    [1171, 529, 1229, 545], [1174, 560, 1227, 579], [1175, 591, 1236, 610],
    [1176, 622, 1231, 638], [1176, 654, 1236, 673],
]
RANKING_LEVEL_BOXES = [
    # The first value has a wider green glow, so retain its previous full removal.
    [1355, 525, 1405, 550], [1367, 560, 1397, 576], [1368, 591, 1397, 607],
    [1370, 623, 1399, 639], [1370, 654, 1399, 670],
]
for index, box in enumerate(RANKING_ROW_BOXES, 1):
    top, bottom = box[1], box[3]
    removed = [RANKING_NAME_BOXES[index - 1], RANKING_LEVEL_BOXES[index - 1],
               CROPS[f"avatar-{index}"][0]]
    removed.append(CROPS[f"medal-{index}"][0] if index <= 3 else [1062, top + 3, 1086, bottom - 3])
    MASKED_CROPS[f"ranking-row-surface-{index}"] = (
        box, removed,
        "Exact row texture/grid; sample name, level, avatar and rank/medal removed; render only for an actual native row",
    )

# Border bands (T,R,B,L), corner squares, and border-image-slice values.
# Corner squares can exceed top strips without retaining any header lettering.
FRAMES = {
    "nav-frame": ([242, 75, 1429, 121], [8, 38, 8, 36], 0, [8, 38, 8, 36]),
    "status-frame": ([1116, 133, 1387, 326], [8, 14, 14, 14], 24, [24, 24, 24, 24]),
    "news-frame": ([125, 458, 1007, 710], [12, 18, 14, 18], 24, [24, 24, 24, 24]),
    "ranking-frame": ([1016, 460, 1455, 701], [8, 12, 12, 12], 20, [20, 20, 20, 20]),
    "feature-frame": (FEATURE_BOXES[0], [7, 12, 12, 12], 12, [12, 12, 12, 12]),
    "button-register-frame": ([598, 275, 825, 330], [10, 18, 12, 18], 18, [18, 18, 18, 18]),
    "button-download-frame": ([834, 275, 1077, 330], [10, 18, 12, 18], 18, [18, 18, 18, 18]),
    "gallery-frame": ([126, 716, 1547, 858], [3, 12, 10, 12], 20, [20, 20, 20, 20]),
}


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def clear_source_rect(image, outer, removed):
    left, top, right, bottom = removed
    x0, y0, x1, y1 = outer
    if not (x0 <= left < right <= x1 and y0 <= top < bottom <= y1):
        raise ValueError(f"Removal rectangle {removed} falls outside {outer}")
    image.paste((0, 0, 0, 0), (left - x0, top - y0, right - x0, bottom - y0))


def frame_pixels(source, box, bands, corner):
    original = source.crop(box)
    width, height = original.size
    top, right, bottom, left = bands
    regions = [(0, 0, width, top), (0, 0, left, height),
               (width - right, 0, width, height), (0, height - bottom, width, height)]
    if corner:
        regions += [(0, 0, corner, corner), (width - corner, 0, width, corner),
                    (0, height - corner, corner, height),
                    (width - corner, height - corner, width, height)]
    output = Image.new("RGBA", original.size, (0, 0, 0, 0))
    for region in regions:
        output.paste(original.crop(region), region)
    if box == FRAMES["nav-frame"][0]:
        # The separate fixed-size ornament must not be stretched by border-image.
        clear_source_rect(output, box, [801, 113, 869, 121])
    return output


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True, help="Owner-provided original PNG; its SHA256 must match the admitted reference")
    parser.add_argument("--output", type=Path, default=HERE.parent / "static/img/reference")
    parser.add_argument("--manifest", type=Path, default=HERE.parent / "static/img/reference/layout.json")
    parser.add_argument("--all", action="store_true", help="Generate optional alternative crops as well as the checked-in selection")
    args = parser.parse_args()
    if sha256(args.source.read_bytes()) != REFERENCE_SHA256:
        raise ValueError("Source SHA256 differs from the owner-provided original; refusing a different design")
    selected = None
    if args.manifest.exists() and not args.all:
        selected = set(json.loads(args.manifest.read_text(encoding="utf-8"))["assets"])
    source = Image.open(args.source).convert("RGBA")
    if source.size != REFERENCE_SIZE:
        raise ValueError(f"Expected reference dimensions {REFERENCE_SIZE}, received {source.size}")
    args.output.mkdir(parents=True, exist_ok=True)
    assets = {}

    def save(name, expected, box, kind, note, extra=None):
        if selected is not None and name not in selected:
            return
        # All nontransparent pixels must remain bit-identical to their source.
        raw = source.crop(box)
        expected_data, source_data = expected.tobytes(), raw.tobytes()
        for offset in range(0, len(expected_data), 4):
            pixel, reference = expected_data[offset:offset + 4], source_data[offset:offset + 4]
            if pixel[3] and pixel != reference:
                raise AssertionError(f"Changed retained source pixel in {name}")
            if not pixel[3] and pixel != b"\x00\x00\x00\x00":
                raise AssertionError(f"Hidden RGB data remains in {name}")
        path = args.output / (name + ".webp")
        expected.save(path, "WEBP", lossless=True, exact=True, method=6)
        decoded = Image.open(path).convert("RGBA")
        if decoded.size != expected.size or decoded.tobytes() != expected.tobytes():
            raise AssertionError(f"Lossless roundtrip failed in {name}")
        assets[name] = {
            "file": path.name, "source_box_xyxy": box, "size": list(expected.size),
            "kind": kind, "note": note, "bytes": path.stat().st_size,
            "sha256": sha256(path.read_bytes()),
            "decoded_rgba_sha256": sha256(decoded.tobytes()),
            "decoded_rgba_matches_expected": True,
            **(extra or {}),
        }

    for name, (box, note) in CROPS.items():
        save(name, source.crop(box), box, "exact-crop", note)
    for name, (box, clear, note) in MASKED_CROPS.items():
        result = source.crop(box)
        for region in clear:
            clear_source_rect(result, box, region)
        save(name, result, box, "crop-with-removed-regions", note, {"removed_source_rectangles": clear})
    for name, (box, bands, corner, slices) in FRAMES.items():
        save(name, frame_pixels(source, box, bands, corner), box, "source-border-only",
             "Only original perimeter pixels; interior is transparent zero RGBA; no dynamic values",
             {"retained_edge_widths_trbl": bands, "retained_corner_size": corner,
              "border_image_slice_trbl": slices, "border_image_width_px_trbl": slices,
              "border_image_repeat": "stretch", "border_image_fill": False})

    manifest = {
        "source": {"filename": args.source.name, "sha256": sha256(args.source.read_bytes()),
                   "size": list(source.size), "mode": "RGBA"},
        "coordinate_convention": "Half-open source [x0,y0,x1,y1]; page origin (0,0); CSS reference 1672x941",
        "asset_url_base_suggestion": "/static/img/reference/",
        "encoding": {"format": "WEBP", "lossless": True, "exact": True, "resampling": False},
        "encoder_versions": {"pillow": pillow_version, "libwebp": features.version("webp")},
        "verification": {"all_retained_pixels_identical_to_source": True,
                         "all_decoded_rgba_identical_to_expected_crop_or_removed_region": True,
                         "hidden_rgb_in_removed_regions": False},
        "nav_hit_boxes_xyxy": [[273, 77, 448, 121], [448, 77, 592, 121], [592, 77, 718, 121],
                               [718, 77, 882, 121], [882, 77, 1017, 121], [1017, 77, 1123, 121],
                               [1123, 77, 1293, 121], [1293, 77, 1398, 121]],
        "nav_order": ["home", "news", "download", "register-or-account", "ranking", "wiki", "community", "shop"],
        "footer_hit_boxes_xyxy": [[454, 860, 572, 904], [573, 860, 679, 904], [680, 860, 763, 904],
                                  [764, 860, 868, 904], [869, 860, 958, 904], [959, 860, 1032, 904],
                                  [1033, 860, 1143, 904], [1144, 860, 1220, 904]],
        "logo_hit_boxes_xyxy": {"header": CROPS["logo-top"][0], "footer": CROPS["footer-logo"][0]},
        "hero_cta_boxes_xyxy": {"register": CROPS["button-register"][0], "download": CROPS["button-download"][0]},
        "dynamic_panel_boxes_xyxy": {name: FRAMES[name + "-frame"][0] for name in ("status", "news", "ranking")},
        "feature_boxes_xyxy": FEATURE_BOXES,
        "gallery_boxes_xyxy": GALLERY_BOXES,
        "gallery_cta_box_xyxy": CROPS["button-community"][0],
        "responsive_notes": [
            "At 1672px place crops at their recorded source coordinates and intrinsic dimensions; feature top overlaps hero by 3px.",
            "Use individual feature/gallery/art crops below reference width; avoid scaling the entire page into unreadable text.",
            "Static Polish raster wording needs matching accessible HTML labels; English and authenticated variants can use native text over the clean source frames/textures.",
            "The source contains no disabled-navigation artwork; do not invent one. Preserve source pixels and express disabled state semantically.",
            "Dynamic status/news/ranking must use actual application state and native text, never the reference sample values.",
            "Neutral texture crops may repeat/stretch; these are original source pixels, not reconstructed backgrounds.",
            "Do not repeat full-height side artwork vertically: it contains architectural/torch landmarks.",
            "Static footer artwork retains source year 2025; use the separate footer logo/texture when rendering current native footer text.",
        ],
        "assets": assets,
    }
    args.manifest.parent.mkdir(parents=True, exist_ok=True)
    args.manifest.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"assets": len(assets), "bytes": sum(a["bytes"] for a in assets.values()),
                      "manifest": str(args.manifest), "source_sha256": manifest["source"]["sha256"],
                      "pixel_equivalence": "PASS"}, indent=2))


if __name__ == "__main__":
    main()
