use super::*;
use sha2::{Digest, Sha256};
pub(super) const TABLES: &[&str] = &[
    "news_posts",
    "managed_pages",
    "events",
    "event_translations",
    "site_announcements",
    "editorial_translations",
    "wiki_articles",
    "wiki_revisions",
    "wiki_article_translations",
    "editorial_media",
    "editorial_media_references",
];
fn translations<'a>(
    data: &'a Dataset,
    kind: &'a str,
    id: &'a str,
) -> impl Iterator<Item = &'a Record> {
    data.get("editorial_translations")
        .into_iter()
        .flatten()
        .filter(move |r| {
            text(r, "content_type").ok().as_deref() == Some(kind)
                && text(r, "content_id").ok().as_deref() == Some(id)
        })
}
pub(super) fn map(ctx: &Context, data: &Dataset, table: &str, row: &Record) -> Result<Vec<Insert>> {
    let mut records = Vec::new();
    match table {
        "news_posts" | "site_announcements" => {
            let (target, kind, fields): (&str, &str, &[&str]) = if table == "news_posts" {
                (
                    "news",
                    "news_post",
                    &["slug", "title", "body", "published_at"],
                )
            } else {
                (
                    "announcements",
                    "site_announcement",
                    &[
                        "title",
                        "body",
                        "severity",
                        "starts_at",
                        "ends_at",
                        "publication_state",
                        "action_label",
                        "action_url",
                        "lock_version",
                    ],
                )
            };
            let mut r = standard(
                ctx,
                table,
                row,
                if target == "news" {
                    "news"
                } else {
                    "announcements"
                },
                fields,
            )?;
            if table == "site_announcements" {
                r.values.remove("created_at");
                r.values.insert(
                    "created_by".into(),
                    ctx.reference(row, "created_by", "identities")?,
                );
                r.values.insert(
                    "updated_by".into(),
                    ctx.reference(row, "updated_by", "identities")?,
                );
            }
            let id = text(row, "id")?;
            for tr in translations(data, kind, &id) {
                if text(tr, "locale")? != "pl" {
                    return Err("unsupported editorial translation locale");
                }
                for field in ["title", "body", "action_label"] {
                    if let Some(value) = tr.get(field) {
                        if table == "news_posts" && field == "action_label" {
                            continue;
                        }
                        r.values.insert(format!("{field}_pl"), value.clone());
                    }
                }
                r.values.insert(
                    "translation_published_at".into(),
                    tr.get("published_at").cloned().unwrap_or(Value::Null),
                );
                r.values.insert(
                    "translation_source_updated_at".into(),
                    tr.get("source_updated_at").cloned().unwrap_or(Value::Null),
                );
            }
            records.push(r);
        }
        "managed_pages" => {
            records.push(standard(ctx, table, row, "pages", &["slug"])?);
            let id = mapped_id(ctx, table, row)?;
            let mut r = insert(
                "editorial_pages",
                json!({"page_id":id,"locale":"en","title":text(row,"title")?,"body":text(row,"body")?,"published_at":row.get("published_at")}),
            )?;
            copy(row, &["created_at", "updated_at"], &mut r.values);
            records.push(r);
            let legacy = text(row, "id")?;
            for tr in translations(data, "managed_page", &legacy) {
                records.push(insert("editorial_pages",json!({"page_id":id,"locale":text(tr,"locale")?,"title":text(tr,"title")?,"body":text(tr,"body")?,"published_at":tr.get("published_at")}))?);
            }
        }
        "events" => {
            let mut r = standard(
                ctx,
                table,
                row,
                "events",
                &["status", "starts_at", "ends_at", "lock_version"],
            )?;
            r.values
                .insert("featured".into(), json!(flag(row, "featured", false)?));
            for col in ["created_by", "updated_by", "published_by"] {
                r.values
                    .insert(col.into(), ctx.reference(row, col, "identities")?);
            }
            records.push(r);
        }
        "event_translations" => {
            let mut r = standard(
                ctx,
                table,
                row,
                "event_translations",
                &["locale", "title", "slug", "summary", "body"],
            )?;
            r.values.remove("id");
            r.values
                .insert("event_id".into(), ctx.reference(row, "event_id", "events")?);
            records.push(r);
        }
        "editorial_translations" => {
            let parent = match text(row, "content_type")?.as_str() {
                "news_post" => "news_posts",
                "managed_page" => "managed_pages",
                "site_announcement" => "site_announcements",
                _ => return Err("unsupported editorial content type"),
            };
            ctx.reference(row, "content_id", parent)?;
            if text(row, "locale")? != "pl" {
                return Err("unsupported editorial locale");
            }
        }
        "wiki_articles" => {
            let status = text(row, "status")?;
            let status = if status == "review" {
                "in_review"
            } else {
                &status
            };
            let version = number(row, "lock_version")?;
            records.push(insert("wiki_guides",json!({"id":mapped_id(ctx,table,row)?,"status":status,"version":version,"revision_version":version,"published_at":row.get("published_at"),"created_at":row.get("created_at"),"updated_at":row.get("updated_at")}))?);
        }
        "wiki_revisions" | "wiki_article_translations" => {
            let article_id = text(row, "article_id")?;
            let article = data
                .get("wiki_articles")
                .into_iter()
                .flatten()
                .find(|r| text(r, "id").ok().as_ref() == Some(&article_id))
                .ok_or("wiki article missing")?;
            let version = if table == "wiki_revisions" {
                number(row, "article_version")?
            } else {
                number(article, "lock_version")?
            };
            if table == "wiki_article_translations"
                && data.get("wiki_revisions").into_iter().flatten().any(|r| {
                    text(r, "article_id").ok().as_ref() == Some(&article_id)
                        && number(r, "article_version").ok() == Some(version)
                        && r.get("locale") == row.get("locale")
                })
            {
                return Ok(records);
            }
            let editor = if table == "wiki_revisions" {
                ctx.reference(row, "editor_identity_id", "identities")?
            } else {
                ctx.reference(article, "last_editor_identity_id", "identities")?
            };
            records.push(insert("wiki_revisions",json!({"id":mapped_id(ctx,table,row)?,"guide_id":mapped_id(ctx,"wiki_articles",article)?,"version":version,"locale":text(row,"locale")?,"title":text(row,"title")?,"slug":text(row,"slug")?,"summary":row.get("summary").cloned().unwrap_or(json!("")),"markdown":text(row,"source_markdown")?,"editor":editor,"change_note":row.get("change_note").filter(|v| !v.is_null()).cloned().unwrap_or(json!("Legacy import")),"created_at":row.get("created_at")}))?);
        }
        "editorial_media" => {
            let root = std::env::var("PORTAL_IMPORT_MEDIA_ROOT")
                .map_err(|_| "media files require a local media root")?;
            let path = text(row, "storage_path")?;
            let content = read_media(&root, &path)?;
            let hash = Sha256::digest(&content);
            if content.len() as i64 != number(row, "byte_size")?
                || hex(&text(row, "sha256")?)? != hash[..]
            {
                return Err("media checksum or size mismatch");
            }
            let processed = oteryn_content::media::process(&path, &content)
                .map_err(|_| "media validation failed")?;
            let mut r = standard(
                ctx,
                table,
                row,
                "media",
                &["alt_text", "mime_type", "width", "height"],
            )?;
            if processed.mime != text(row, "mime_type")?
                || processed.width as i64 != number(row, "width")?
                || processed.height as i64 != number(row, "height")?
            {
                return Err("media dimensions mismatch");
            }
            r.values.insert("content".into(), bytes(&content));
            r.values.insert("sha256".into(), bytes(&hash));
            if let Some(Value::String(path)) = row.get("thumbnail_path") {
                let thumb = read_media(&root, path)?;
                let hash = Sha256::digest(&thumb);
                if hex(&text(row, "thumbnail_sha256")?)? != hash[..] {
                    return Err("thumbnail checksum mismatch");
                }
                oteryn_content::media::process(path, &thumb)
                    .map_err(|_| "thumbnail validation failed")?;
                r.values.insert("thumbnail".into(), bytes(&thumb));
                r.values.insert("thumbnail_sha256".into(), bytes(&hash));
            }
            records.push(r);
        }
        "editorial_media_references" => {
            records.push(insert("media_references",json!({"media_id":ctx.reference(row,"media_id","editorial_media")?,"consumer":format!("{}:{}:{}",text(row,"consumer")?,text(row,"consumer_id")?,text(row,"usage")?)}))?);
        }
        _ => return Err("unsupported content table"),
    }
    Ok(records)
}
fn read_media(root: &str, path: &str) -> Result<Vec<u8>> {
    use std::path::{Component, Path};
    if Path::new(path)
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err("invalid media path");
    }
    let root = Path::new(root)
        .canonicalize()
        .map_err(|_| "media root unavailable")?;
    let path = root
        .join(path)
        .canonicalize()
        .map_err(|_| "media file missing")?;
    if !path.starts_with(root) {
        return Err("media path escapes root");
    }
    if std::fs::metadata(&path)
        .map_err(|_| "media file unavailable")?
        .len()
        > 8388608
    {
        return Err("media file too large");
    }
    std::fs::read(path).map_err(|_| "media file unavailable")
}
