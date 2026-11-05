//! Design-system integration tests discovered by the workspace test suite.
use askama::Template;
use std::{collections::BTreeMap, error::Error, fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

type Palette = BTreeMap<String, [u8; 3]>;
type Pair = (String, String, f64);

fn rgb(value: &str) -> Result<[u8; 3], String> {
    if value.len() != 7 || !value.starts_with('#') || !value[1..].is_ascii() {
        return Err(format!("expected six-digit hex color: {value}"));
    }
    let channel =
        |start| u8::from_str_radix(&value[start..start + 2], 16).map_err(|e| e.to_string());
    Ok([channel(1)?, channel(3)?, channel(5)?])
}

fn palette(css: &str, scope: &str) -> Result<Palette, String> {
    let start = css
        .find(scope)
        .ok_or_else(|| format!("missing scope {scope}"))?;
    let body = css[start + scope.len()..]
        .split('}')
        .next()
        .ok_or("missing body")?;
    let mut colors = Palette::new();
    for declaration in body.split(';') {
        if let Some((name, value)) = declaration.trim().split_once(':')
            && let Some(name) = name.trim().strip_prefix("--op-")
        {
            let value = value.trim();
            // Shadows contain embedded colors, but are never text surfaces.
            if value.starts_with('#') && colors.insert(name.to_owned(), rgb(value)?).is_some() {
                return Err(format!("duplicate color: {name}"));
            }
        }
    }
    if colors.is_empty() {
        return Err("empty palette".to_owned());
    }
    Ok(colors)
}

fn pairs(css: &str) -> Result<Vec<Pair>, String> {
    let mut result = Vec::new();
    for line in css.lines() {
        if let Some(pair) = line.trim().strip_prefix("@contrast ") {
            let parts: Vec<_> = pair.split_whitespace().collect();
            if parts.len() != 3 {
                return Err("contrast declaration must have three fields".to_owned());
            }
            let minimum: f64 = parts[2].parse().map_err(|_| "invalid contrast threshold")?;
            if minimum != 3.0 && minimum != 4.5 {
                return Err("only AA text (4.5) or non-text (3) thresholds are allowed".to_owned());
            }
            if result
                .iter()
                .any(|(fg, bg, _)| fg == parts[0] && bg == parts[1])
            {
                return Err("duplicate contrast pair".to_owned());
            }
            result.push((parts[0].to_owned(), parts[1].to_owned(), minimum));
        }
    }
    if result.is_empty() {
        return Err("no declared contrast pairs".to_owned());
    }
    Ok(result)
}

fn luminance(color: [u8; 3]) -> f64 {
    let linear = |channel: u8| {
        let srgb = f64::from(channel) / 255.0;
        if srgb <= 0.04045 {
            srgb / 12.92
        } else {
            ((srgb + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(color[0]) + 0.7152 * linear(color[1]) + 0.0722 * linear(color[2])
}

fn contrast(fg: [u8; 3], bg: [u8; 3]) -> f64 {
    let a = luminance(fg);
    let b = luminance(bg);
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

fn verify(palette: &Palette, pair: &Pair) -> Result<f64, String> {
    let fg = palette
        .get(&pair.0)
        .ok_or_else(|| format!("unknown foreground {}", pair.0))?;
    let bg = palette
        .get(&pair.1)
        .ok_or_else(|| format!("unknown background {}", pair.1))?;
    let ratio = contrast(*fg, *bg);
    if ratio < pair.2 {
        return Err(format!("{} / {}: {ratio:.3} < {}", pair.0, pair.1, pair.2));
    }
    Ok(ratio)
}

#[test]
fn all_declared_pairs_meet_aa_in_both_themes() -> Result<(), Box<dyn Error>> {
    let css = fs::read_to_string(root().join("static/css/tokens.css"))?;
    let pairs = pairs(&css)?;
    let dark = palette(&css, "[data-theme=\"dark\"] {")?;
    let light = palette(&css, "[data-theme=\"light\"] {")?;
    assert_eq!(palette(&css, ":root {")?, dark, "default theme drift");
    assert_eq!(
        palette(&css, ":root:not([data-theme=\"dark\"]) {")?,
        light,
        "system light theme drift"
    );
    // Removing a difficult combination from the manifest must fail, too.
    for fg in ["text", "muted", "accent", "danger", "focus", "border"] {
        for bg in ["canvas", "surface", "raised"] {
            let minimum = if fg == "focus" || fg == "border" {
                3.0
            } else {
                4.5
            };
            assert!(
                pairs.contains(&(fg.to_owned(), bg.to_owned(), minimum)),
                "missing {fg}/{bg}"
            );
        }
    }
    for (fg, bg) in [
        ("on-accent", "accent"),
        ("success", "success-bg"),
        ("warning", "warning-bg"),
        ("danger", "danger-bg"),
        ("info", "info-bg"),
    ] {
        assert!(pairs.contains(&(fg.to_owned(), bg.to_owned(), 4.5)));
    }
    let readme = fs::read_to_string(root().join("static/design/README.md"))?;
    for pair in &pairs {
        let dark_ratio = verify(&dark, pair)?;
        let light_ratio = verify(&light, pair)?;
        let row = format!(
            "| {} / {} | {dark_ratio:.2} | {light_ratio:.2} | {:.1} |",
            pair.0, pair.1, pair.2
        );
        assert!(readme.contains(&row), "contrast table needs update: {row}");
    }
    println!(
        "{} pairs × 2 themes: all AA text and non-text checks passed",
        pairs.len()
    );
    Ok(())
}

#[test]
fn refuses_low_contrast_including_near_boundary() -> Result<(), Box<dyn Error>> {
    let palette = Palette::from([
        ("text".to_owned(), rgb("#777777")?),
        ("surface".to_owned(), rgb("#ffffff")?),
    ]);
    assert!(verify(&palette, &("text".to_owned(), "surface".to_owned(), 4.5)).is_err());
    assert!((contrast([0, 0, 0], [255, 255, 255]) - 21.0).abs() < 0.0001);
    assert_eq!(contrast([42, 42, 42], [42, 42, 42]), 1.0);
    Ok(())
}

#[test]
fn rejects_bad_or_missing_color_and_pair_input() {
    for bad in ["#fff", "#ffffff00", "#gg0000", "ffffff", "#é0000", ""] {
        assert!(rgb(bad).is_err());
    }
    for bad in [
        "",
        "@contrast text",
        "@contrast text surface NaN",
        "@contrast text surface 1",
        "@contrast text surface 4.5\n@contrast text surface 4.5",
    ] {
        assert!(pairs(bad).is_err());
    }
    assert!(palette("[data-theme] { --op-text: #zzzzzz; }", "[data-theme] {").is_err());
    assert!(palette("", ":root {").is_err());
    assert!(
        verify(
            &Palette::new(),
            &("missing".to_owned(), "surface".to_owned(), 4.5)
        )
        .is_err()
    );
}

struct Theme {
    name: &'static str,
    lang: &'static str,
    copy: BTreeMap<String, String>,
}

#[derive(Template)]
#[template(path = "components/gallery.html")]
struct Gallery {
    themes: Vec<Theme>,
}

fn catalogue(lang: &str) -> Result<BTreeMap<String, String>, Box<dyn Error>> {
    let source = fs::read_to_string(root().join(format!("static/design/locales/{lang}.tsv")))?;
    let mut result = BTreeMap::new();
    for line in source
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
    {
        let (key, value) = line.split_once('\t').ok_or("malformed translation row")?;
        if result.insert(key.to_owned(), value.to_owned()).is_some() {
            return Err("duplicate translation key".into());
        }
    }
    Ok(result)
}

#[test]
fn partials_compile_and_bilingual_gallery_matches() -> Result<(), Box<dyn Error>> {
    assert!(
        fs::metadata(root().join("static/design/drawer.js"))?.len() < 5000,
        "drawer script must stay below 5 KB"
    );
    let en = catalogue("en")?;
    let pl = catalogue("pl")?;
    assert_eq!(en.keys().collect::<Vec<_>>(), pl.keys().collect::<Vec<_>>());
    let html = Gallery {
        themes: vec![
            Theme {
                name: "dark",
                lang: "en",
                copy: en,
            },
            Theme {
                name: "light",
                lang: "pl",
                copy: pl,
            },
        ],
    }
    .render()?;
    let html = html
        .trim_start()
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let path = root().join("static/design/index.html");
    if std::env::var_os("UPDATE_DESIGN_GALLERY").is_some() {
        fs::write(&path, &html)?;
    }
    assert_eq!(
        html,
        fs::read_to_string(path)?,
        "run UPDATE_DESIGN_GALLERY=1 cargo test --locked -p oteryn-web --test design_system partials_compile_and_bilingual_gallery_matches after changing partials or gallery copy"
    );
    for component in [
        "op-button",
        "op-link",
        "op-field",
        "op-alert",
        "op-card",
        "op-table",
        "op-tabs",
        "op-pagination",
        "op-badge",
        "op-breadcrumbs",
        "op-disclosure",
        "op-panel",
        "op-frame",
        "op-empty",
        "op-loading",
    ] {
        assert!(
            html.matches(component).count() >= 2,
            "missing both themes for {component}"
        );
    }
    Ok(())
}

#[derive(Template)]
#[template(
    source = "{% import \"components/field.html\" as f %}{% import \"components/button.html\" as b %}{% import \"components/alert.html\" as a %}{% call f::field(\"name\", label, value, \"Hint\", error, true) %}{% endcall %}{% call b::button(\"Unavailable\", false, true) %}{% endcall %}{% call a::alert(\"Refused\", error, \"danger\") %}{% endcall %}",
    ext = "html"
)]
struct InvalidField<'a> {
    label: &'a str,
    value: &'a str,
    error: &'a str,
}

#[test]
fn invalid_input_is_escaped_and_refusal_is_accessible() -> Result<(), Box<dyn Error>> {
    let html = InvalidField {
        label: "<script>alert(1)</script>",
        value: "\" autofocus onfocus=\"alert(1)",
        error: "<img src=x onerror=alert(1)>",
    }
    .render()?;
    assert!(!html.contains("<script>"));
    assert!(!html.contains("<img"));
    assert!(!html.contains("value=\"\" autofocus"));
    assert!(html.contains("aria-invalid=\"true\""));
    assert!(html.contains("aria-describedby=\"name-hint name-error\""));
    assert!(html.contains(" disabled>"));
    assert!(html.contains("role=\"alert\""));
    Ok(())
}
