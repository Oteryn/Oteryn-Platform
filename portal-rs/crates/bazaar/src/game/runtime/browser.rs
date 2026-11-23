use super::*;
use crate::{Evidence, State};
use askama::Template;
use axum::{
    Extension, Form as BrowserForm, Router,
    extract::{Path, Query, State as HttpState},
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use oteryn_web::session::{BrowserSession, SessionLayer};
use sha2::{Digest, Sha256};
#[derive(Debug)]
struct Card {
    id: Uuid,
    name: String,
    snapshot: Option<crate::characters::SaleSnapshot>,
    publication: Option<crate::items::Publication>,
    quantity: i64,
    ends_at: String,
    kind: String,
    state: String,
    price: i64,
    currency: String,
    owner: bool,
    refused: bool,
    completed: bool,
    cancelled: bool,
}
#[derive(Template)]
#[template(
    source = "<!doctype html><html lang=\"{{ lang }}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{{ title }}</title></head><body><header><a href=\"/{{ lang }}/bazaar\">{{ title }}</a> <a href=\"/{{ lang }}/account/bazaar\">{{ own_label }}</a> <a href=\"/{{ lang }}/account/bazaar/sell\">{{ sell_label }}</a> <a href=\"/{{ lang }}/bazaar/readiness\">{{ self.readiness_label() }}</a></header><main><h1>{{ title }}</h1>{% if !message.is_empty() %}<p role=\"status\">{{ message }}</p>{% endif %}{% if create %}<form method=\"post\" action=\"/{{ lang }}/account/bazaar/sell\"><input type=\"hidden\" name=\"_csrf\" value=\"{{ csrf }}\"><input type=\"hidden\" name=\"request_id\" value=\"{{ request }}\"><label>{{ kind_label }}<select name=\"kind\"><option value=\"character\">{{ character_label }}</option><option value=\"item\">{{ item_label }}</option></select></label><label>{{ world_label }}<input required name=\"world_id\"></label><label>{{ character_label }} ID<input required name=\"seller_character_id\"></label><label>{{ asset_label }}<input required name=\"asset_id\"></label><label>{{ revision_label }}<input name=\"expected_revision\" required value=\"7\"></label><label>{{ quantity_label }}<input name=\"quantity\" required type=\"number\" min=\"1\" value=\"1\"></label><label>{{ definition_label }}<input name=\"definition_key\" value=\"item\"></label><label>{{ version_label }}<input name=\"definition_revision\" value=\"v1\"></label><label>{{ currency_label }}<select name=\"currency\"><option value=\"COINS\">Coins</option><option value=\"GOLD\">Gold</option></select></label><label>{{ price_label }}<input name=\"starting_bid\" required type=\"number\" min=\"1\"></label><label>{{ days_label }}<input name=\"duration_days\" required type=\"number\" min=\"1\" max=\"14\" value=\"1\"></label><label><input type=\"checkbox\" name=\"publish_snapshot\" value=\"true\" required>{{ consent_label }}</label><button type=\"submit\">{{ sell_label }}</button></form>{% endif %}{% for card in cards %}<article data-listing=\"{{ card.id }}\" data-state=\"{{ card.state }}\"><h2><a href=\"/{{ lang }}/bazaar/{{ card.id }}\">{{ card.name }}</a></h2><p>{% if card.kind == \"character\" %}{{ character_label }}{% else %}{{ item_label }}{% endif %} · {{ card.price }} {{ card.currency }}</p><p role=\"status\">{% if card.refused %}{{ refused_label }}{% elif card.cancelled %}{{ cancelled_label }}{% elif card.state == \"finalizing\" %}{{ self.text(\"finalizing\") }}{% elif card.completed %}{{ complete_label }}{% elif card.state == \"active\" %}{{ active_label }}{% else %}{{ pending_label }}{% endif %}</p><p>{{ self.text(\"deadline\") }}: {{ card.ends_at }}</p>{% if detail %}{% if let Some(snapshot) = card.snapshot %}<dl><dt>{{ self.text(\"world\") }}</dt><dd>{{ snapshot.world }}</dd><dt>{{ self.text(\"level\") }}</dt><dd>{{ snapshot.level }}</dd><dt>{{ self.text(\"vocation\") }}</dt><dd>{{ snapshot.vocation }}</dd><dt>{{ self.text(\"health\") }}</dt><dd>{{ snapshot.health }}</dd><dt>{{ self.text(\"health_max\") }}</dt><dd>{{ snapshot.health_max }}</dd><dt>{{ self.text(\"mana\") }}</dt><dd>{{ snapshot.mana }}</dd><dt>{{ self.text(\"mana_max\") }}</dt><dd>{{ snapshot.mana_max }}</dd><dt>{{ self.text(\"capacity_centi_oz\") }}</dt><dd>{{ snapshot.capacity_centi_oz }}</dd><dt>{{ self.text(\"speed\") }}</dt><dd>{{ snapshot.speed }}</dd><dt>{{ self.text(\"experience\") }}</dt><dd>{{ snapshot.experience }}</dd><dt>{{ self.text(\"snapshot_hash\") }}</dt><dd>{{ snapshot.snapshot_hash }}</dd><dt>{{ self.text(\"captured_at\") }}</dt><dd><time data-unix=\"{{ snapshot.captured_at }}\">{{ self.timestamp(snapshot.captured_at) }}</time></dd>{% if let Some(value) = snapshot.created_at %}<dt>{{ self.text(\"created_at\") }}</dt><dd>{{ self.timestamp(value) }}</dd>{% endif %}{% if let Some(value) = snapshot.world_transfer_eligible %}<dt>{{ self.text(\"world_transfer_eligible\") }}</dt><dd>{{ value }}</dd>{% endif %}</dl><h3>{{ self.text(\"skills\") }}</h3><ul>{% for skill in snapshot.skills %}<li>{{ skill.key }}: {{ skill.level }} ({{ self.progress(skill.progress_bps) }})</li>{% endfor %}</ul><ul>{% for value in snapshot.blessings %}<li>{{ value }}</li>{% endfor %}</ul>{% if let Some(values) = snapshot.outfits %}<section><h3>{{ self.text(\"outfits\") }}</h3><ul>{% for value in values %}<li>{{ value.key }}: {{ value.value }}</li>{% endfor %}</ul></section>{% endif %}{% if let Some(values) = snapshot.mounts %}<section><h3>{{ self.text(\"mounts\") }}</h3><ul>{% for value in values %}<li>{{ value.key }}: {{ value.value }}</li>{% endfor %}</ul></section>{% endif %}{% if let Some(values) = snapshot.achievements %}<section><h3>{{ self.text(\"achievements\") }}</h3><ul>{% for value in values %}<li>{{ value.key }}: {{ value.value }}</li>{% endfor %}</ul></section>{% endif %}{% if let Some(values) = snapshot.quests %}<section><h3>{{ self.text(\"quests\") }}</h3><ul>{% for value in values %}<li>{{ value.key }}: {{ value.value }}</li>{% endfor %}</ul></section>{% endif %}{% if let Some(values) = snapshot.items %}<section><h3>{{ self.text(\"items\") }}</h3><ul>{% for value in values %}<li>{{ value.key }}: {{ value.quantity }}</li>{% endfor %}</ul></section>{% endif %}{% if let Some(values) = snapshot.store_grants %}<section><h3>{{ self.text(\"store_grants\") }}</h3><ul>{% for value in values %}<li>{{ value }}</li>{% endfor %}</ul></section>{% endif %}{% endif %}{% if let Some(item) = card.publication %}<dl><dt>{{ quantity_label }}</dt><dd>{{ card.quantity }}</dd>{% for (key,value) in item.attributes %}<dt>{{ self.text(key) }}</dt><dd>{{ value }}</dd>{% endfor %}</dl>{% endif %}{% endif %}{% if detail && card.owner && !card.completed && !card.refused %}<form method=\"post\" action=\"/{{ lang }}/account/bazaar/{{ card.id }}/control\"><input type=\"hidden\" name=\"_csrf\" value=\"{{ csrf }}\"><input type=\"hidden\" name=\"request_id\" value=\"{{ request }}\"><button name=\"action\" value=\"resume\">{{ resume_label }}</button><button name=\"action\" value=\"cancel\">{{ cancel_label }}</button></form>{% endif %}{% if detail && card.state == \"active\" && !card.owner %}<form method=\"post\" action=\"/{{ lang }}/account/bazaar/{{ card.id }}/bids\"><input type=\"hidden\" name=\"_csrf\" value=\"{{ csrf }}\"><input type=\"hidden\" name=\"request_id\" value=\"{{ request }}\"><label>{{ destination_label }}<input name=\"buyer_character_id\" required></label><label>{{ maximum_label }}<input name=\"maximum\" type=\"number\" min=\"1\" required></label><button type=\"submit\">{{ bid_label }}</button></form>{% endif %}</article>{% endfor %}{% if !next.is_empty() %}<a rel=\"next\" href=\"?before={{ next }}\">{{ self.text(\"next\") }}</a>{% endif %}</main></body></html>",
    ext = "html"
)]
struct Page<'a> {
    lang: &'a str,
    title: String,
    own_label: String,
    sell_label: String,
    resume_label: String,
    cancel_label: String,
    kind_label: String,
    character_label: String,
    item_label: String,
    world_label: String,
    asset_label: String,
    revision_label: String,
    quantity_label: String,
    definition_label: String,
    version_label: String,
    price_label: String,
    currency_label: String,
    days_label: String,
    consent_label: String,
    refused_label: String,
    complete_label: String,
    cancelled_label: String,
    active_label: String,
    pending_label: String,
    destination_label: String,
    maximum_label: String,
    bid_label: String,
    message: String,
    csrf: String,
    request: Uuid,
    create: bool,
    detail: bool,
    cards: Vec<Card>,
    next: String,
}
fn translated(lang: &str, key: &str) -> String {
    let Ok(id) = lang.parse::<unic_langid::LanguageIdentifier>() else {
        return String::new();
    };
    let mut bundle = fluent_bundle::FluentBundle::new(vec![id]);
    let source = if lang == "pl" {
        include_str!("pl.ftl")
    } else {
        include_str!("en.ftl")
    };
    let Ok(resource) = fluent_bundle::FluentResource::try_new(source.into()) else {
        return String::new();
    };
    if bundle.add_resource(resource).is_err() {
        return String::new();
    }
    bundle
        .get_message(key)
        .and_then(|m| m.value())
        .map(|v| bundle.format_pattern(v, None, &mut vec![]).into_owned())
        .unwrap_or_default()
}
impl<'a> Page<'a> {
    fn readiness_label(&self) -> String {
        crate::characters::http::text(self.lang, "readiness-title")
    }

    fn text(&self, key: &str) -> String {
        translated(self.lang, key)
    }
    fn progress(&self, bps: &u16) -> String {
        format!("{}.{:02}%", bps / 100, bps % 100)
    }
    fn timestamp(&self, value: &u64) -> String {
        i64::try_from(*value)
            .ok()
            .and_then(|seconds| OffsetDateTime::from_unix_timestamp(seconds).ok())
            .map_or_else(|| value.to_string(), |time| time.to_string())
    }
    fn new(lang: &'a str) -> Self {
        Self {
            lang,
            title: translated(lang, "title"),
            own_label: translated(lang, "own"),
            resume_label: translated(lang, "resume"),
            cancel_label: translated(lang, "cancel"),
            sell_label: translated(lang, "sell"),
            kind_label: translated(lang, "kind"),
            character_label: translated(lang, "character"),
            item_label: translated(lang, "item"),
            world_label: translated(lang, "world"),
            asset_label: translated(lang, "asset"),
            revision_label: translated(lang, "revision"),
            quantity_label: translated(lang, "quantity"),
            definition_label: translated(lang, "definition"),
            version_label: translated(lang, "version"),
            currency_label: translated(lang, "currency"),
            price_label: translated(lang, "price"),
            days_label: translated(lang, "days"),
            consent_label: translated(lang, "consent"),
            refused_label: translated(lang, "refused"),
            cancelled_label: translated(lang, "cancelled"),
            complete_label: translated(lang, "complete"),
            active_label: translated(lang, "active"),
            pending_label: translated(lang, "pending"),
            destination_label: translated(lang, "destination"),
            maximum_label: translated(lang, "maximum"),
            bid_label: translated(lang, "bid"),
            message: String::new(),
            csrf: String::new(),
            request: oteryn_core::new_id(),
            create: false,
            detail: false,
            cards: vec![],
            next: String::new(),
        }
    }
}
fn answer(page: Page<'_>, status: StatusCode) -> Response {
    match page.render() {
        Ok(body) => (
            status,
            [(header::CACHE_CONTROL, "no-store, private")],
            Html(body),
        )
            .into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
fn failure(lang: &str, e: Error) -> Response {
    let mut page = Page::new(lang);
    page.message = translated(
        lang,
        if matches!(e, Error::Unavailable | Error::Database(_) | Error::Backlog) {
            "unavailable"
        } else {
            "denied"
        },
    );
    answer(
        page,
        match e {
            Error::Unavailable | Error::Database(_) | Error::Backlog => {
                StatusCode::SERVICE_UNAVAILABLE
            }
            Error::Invalid => StatusCode::UNPROCESSABLE_ENTITY,
            Error::NotFound => StatusCode::NOT_FOUND,
            Error::Conflict | Error::CustodyDrift => StatusCode::CONFLICT,
            _ => StatusCode::FORBIDDEN,
        },
    )
}
fn locale(lang: &str) -> Result<(), Error> {
    if matches!(lang, "pl" | "en") {
        Ok(())
    } else {
        Err(Error::NotFound)
    }
}
impl Runtime {
    pub fn browser_router(&self) -> Router {
        let mut sessions = SessionLayer::new(self.pool.clone());
        sessions.csrf.trusted_origins = vec![self.config.public_origin.clone()];
        Router::new()
            .route("/{locale}/bazaar", get(index))
            .route("/{locale}/bazaar/items", get(index))
            .route("/{locale}/account/bazaar", get(own))
            .route("/{locale}/account/bazaar/sell", get(new).post(create))
            .route("/{locale}/bazaar/items/new", get(new))
            .route("/{locale}/bazaar/{listing}", get(detail))
            .route("/{locale}/bazaar/items/{listing}", get(detail))
            .route("/{locale}/account/bazaar/{listing}/bids", post(bid))
            .route("/{locale}/account/bazaar/{listing}/control", post(control))
            .with_state(self.clone())
            .layer(axum::middleware::from_fn_with_state(
                oteryn_web::ratelimit::RateLimiter::new(self.pool.clone()),
                rate,
            ))
            .layer(axum::middleware::from_fn_with_state(
                sessions,
                oteryn_web::session::middleware,
            ))
            .merge(crate::qualification::public_router())
    }
    async fn cards(
        &self,
        actor: Option<Uuid>,
        listing: Option<Uuid>,
        only_own: bool,
        cursor: Option<(OffsetDateTime, Uuid)>,
    ) -> Result<(Vec<Card>, String), Error> {
        self.config.current()?;
        let mut tx = self.pool.begin().await?;
        crate::locking::listing_gate(&mut tx).await?;
        let mut cards = vec![];
        let rows = candidate_page(&mut tx, actor, listing, only_own, cursor).await?;
        let next = if rows.len() == 50 {
            rows.last()
                .map(|r| format!("{}:{}", r.created_at.unix_timestamp_nanos(), r.id))
                .unwrap_or_default()
        } else {
            String::new()
        };
        tx.commit().await?;
        // Bound work to one candidate page; release issuer locks between candidates.
        for candidate in rows {
            let mut tx = self.pool.begin().await?;
            crate::locking::listing_gate(&mut tx).await?;
            sqlx::query("SET LOCAL statement_timeout='2s'")
                .execute(&mut *tx)
                .await?;
            let row = candidate.row;
            let owner = actor == Some(row.owner);
            let refused: bool = sqlx::query_scalar(
                "SELECT refusal_code IS NOT NULL FROM bazaar_authority_assets WHERE listing_id=$1",
            )
            .bind(row.id)
            .fetch_one(&mut *tx)
            .await?;
            let completed = row.state == State::Completed;
            let buyer: bool = if let Some(actor) = actor {
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_settlements WHERE listing_id=$1 AND buyer=$2 UNION ALL SELECT 1 FROM bazaar_authority_gold_sales WHERE listing_id=$1 AND buyer=$2)").bind(row.id).bind(actor).fetch_one(&mut *tx).await?
            } else {
                false
            };
            let eligible = row.state == State::Active
                && self
                    .authority()
                    .escrow_ready(&mut tx, &row)
                    .await
                    .map_err(core_error)?;
            if !eligible && !owner && !buyer {
                continue;
            }
            let finalizing = completed
                && row.kind == Kind::Character
                && !self
                    .latest(&mut tx, row.id, Command::Unlock)
                    .await?
                    .is_some_and(|(_, receipt)| receipt.committed);
            let mut snapshot = None;
            let mut publication = None;
            let mut quantity = 0;
            let name = if eligible && row.kind == Kind::Character {
                use crate::characters::CharacterEvidence;
                let approved = self
                    .authority()
                    .approved(&mut tx, &row)
                    .await
                    .map_err(core_error)?
                    .ok_or(Error::Unavailable)?;
                let name = approved.name.clone();
                snapshot = Some(approved);
                name
            } else if eligible {
                use crate::items::ItemAuthority;
                let approved = self
                    .authority()
                    .publication(&mut tx, &row)
                    .await
                    .map_err(core_error)?
                    .ok_or(Error::Unavailable)?;
                quantity = sqlx::query_scalar(
                    "SELECT quantity FROM bazaar_authority_assets WHERE listing_id=$1",
                )
                .bind(row.id)
                .fetch_one(&mut *tx)
                .await?;
                let name = approved.name.clone();
                publication = Some(approved);
                name
            } else {
                translated(
                    "en",
                    if row.kind == Kind::Character {
                        "character"
                    } else {
                        "item"
                    },
                )
            };
            tx.commit().await?;
            cards.push(Card {
                snapshot,
                publication,
                quantity,
                ends_at: row.ends_at.to_string(),
                id: row.id,
                name,
                kind: if row.kind == Kind::Character {
                    "character".into()
                } else {
                    "item".into()
                },
                state: if finalizing {
                    "finalizing".into()
                } else {
                    format!("{:?}", row.state).to_ascii_lowercase()
                },
                price: row.price,
                currency: if row.currency == Currency::Gold {
                    "Gold"
                } else {
                    "Coins"
                }
                .into(),
                owner,
                refused,
                completed,
                cancelled: row.state == State::Cancelled,
            });
        }
        if listing.is_some() && cards.is_empty() {
            return Err(Error::NotFound);
        }
        Ok((cards, next))
    }
}
#[derive(Default, Deserialize)]
struct CatalogueQuery {
    before: Option<String>,
}
impl CatalogueQuery {
    fn cursor(&self) -> Result<Option<(OffsetDateTime, Uuid)>, Error> {
        self.before
            .as_deref()
            .map(|v| {
                let (time, id) = v.split_once(':').ok_or(Error::Invalid)?;
                Ok((
                    OffsetDateTime::from_unix_timestamp_nanos(
                        time.parse().map_err(|_| Error::Invalid)?,
                    )
                    .map_err(|_| Error::Invalid)?,
                    id.parse().map_err(|_| Error::Invalid)?,
                ))
            })
            .transpose()
    }
}
async fn index(
    Query(query): Query<CatalogueQuery>,
    HttpState(runtime): HttpState<Runtime>,
    Path(lang): Path<String>,
    Extension(session): Extension<BrowserSession>,
) -> Response {
    if let Err(e) = locale(&lang) {
        return failure("en", e);
    }
    let mut page = Page::new(&lang);
    let cursor = match query.cursor() {
        Ok(c) => c,
        Err(e) => return failure(&lang, e),
    };
    match runtime
        .cards(session.account_id().await, None, false, cursor)
        .await
    {
        Ok((cards, next)) => {
            page.cards = cards;
            page.next = next;
        }
        Err(e) => return failure(&lang, e),
    };
    answer(page, StatusCode::OK)
}
async fn own(
    Query(query): Query<CatalogueQuery>,
    HttpState(runtime): HttpState<Runtime>,
    Path(lang): Path<String>,
    Extension(session): Extension<BrowserSession>,
) -> Response {
    if let Err(e) = locale(&lang) {
        return failure("en", e);
    }
    let Some(actor) = session.account_id().await else {
        return failure(&lang, Error::Denied);
    };
    let mut page = Page::new(&lang);
    let cursor = match query.cursor() {
        Ok(c) => c,
        Err(e) => return failure(&lang, e),
    };
    match runtime.cards(Some(actor), None, true, cursor).await {
        Ok((cards, next)) => {
            page.cards = cards;
            page.next = next;
        }
        Err(e) => return failure(&lang, e),
    };
    answer(page, StatusCode::OK)
}
async fn detail(
    HttpState(runtime): HttpState<Runtime>,
    Path((lang, id)): Path<(String, Uuid)>,
    Extension(session): Extension<BrowserSession>,
) -> Response {
    if let Err(e) = locale(&lang) {
        return failure("en", e);
    }
    let mut page = Page::new(&lang);
    page.detail = true;
    page.csrf = session.csrf_token().await.unwrap_or_default();
    match runtime
        .cards(session.account_id().await, Some(id), false, None)
        .await
    {
        Ok((cards, next)) => {
            page.cards = cards;
            page.next = next;
        }
        Err(e) => return failure(&lang, e),
    };
    answer(page, StatusCode::OK)
}
async fn new(Path(lang): Path<String>, Extension(session): Extension<BrowserSession>) -> Response {
    if let Err(e) = locale(&lang) {
        return failure("en", e);
    }
    if session.account_id().await.is_none() {
        return failure(&lang, Error::Denied);
    }
    let mut page = Page::new(&lang);
    page.create = true;
    page.csrf = session.csrf_token().await.unwrap_or_default();
    answer(page, StatusCode::OK)
}
async fn create(
    HttpState(runtime): HttpState<Runtime>,
    Path(lang): Path<String>,
    headers: HeaderMap,
    Extension(session): Extension<BrowserSession>,
    BrowserForm(input): BrowserForm<NewAuction>,
) -> Response {
    if let Err(e) = locale(&lang) {
        return failure("en", e);
    }
    let Some(actor) = session.account_id().await else {
        return failure(&lang, Error::Denied);
    };
    let hash = match session_hash(&headers) {
        Ok(hash) => hash,
        Err(e) => return failure(&lang, e),
    };
    match runtime.create(actor, hash, input).await {
        Ok(id) => Redirect::to(&format!("/{lang}/bazaar/{id}")).into_response(),
        Err(e) => failure(&lang, e),
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Bid {
    request_id: Uuid,
    maximum: i64,
    buyer_character_id: Uuid,
    #[serde(rename = "_csrf")]
    _csrf: Option<String>,
}
async fn bid(
    HttpState(runtime): HttpState<Runtime>,
    Path((lang, id)): Path<(String, Uuid)>,
    Extension(session): Extension<BrowserSession>,
    BrowserForm(input): BrowserForm<Bid>,
) -> Response {
    if let Err(e) = locale(&lang) {
        return failure("en", e);
    }
    let Some(actor) = session.account_id().await else {
        return failure(&lang, Error::Denied);
    };
    match runtime
        .bid(
            actor,
            id,
            input.maximum,
            input.buyer_character_id,
            input.request_id,
        )
        .await
    {
        Ok(()) => Redirect::to(&format!("/{lang}/bazaar/{id}")).into_response(),
        Err(e) => failure(&lang, e),
    }
}
fn session_hash(headers: &HeaderMap) -> Result<[u8; 32], Error> {
    let mut value = None;
    for header in headers.get_all(header::COOKIE) {
        for cookie in header.to_str().map_err(|_| Error::Denied)?.split(';') {
            if let Some((name, token)) = cookie.trim().split_once('=')
                && name == oteryn_web::session::COOKIE_NAME
            {
                if value.is_some()
                    || token.len() != 64
                    || !token.bytes().all(|b| b.is_ascii_hexdigit())
                {
                    return Err(Error::Denied);
                }
                value = Some(Sha256::digest(token.as_bytes()).into());
            }
        }
    }
    value.ok_or(Error::Denied)
}
async fn rate(
    HttpState(limiter): HttpState<oteryn_web::ratelimit::RateLimiter>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    if request.method() != axum::http::Method::POST {
        return next.run(request).await;
    }
    let Some(session) = request.extensions().get::<BrowserSession>() else {
        return failure("en", Error::Denied);
    };
    let Some(actor) = session.account_id().await else {
        return failure("en", Error::Denied);
    };
    let Some(peer) = request
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
    else {
        return failure("en", Error::Denied);
    };
    let source = match oteryn_web::ratelimit::client_source(peer.0, request.headers()) {
        Ok(source) => source,
        Err(_) => return failure("en", Error::Denied),
    };
    let mut keys = oteryn_web::ratelimit::Keys::source(source);
    keys.account = Some(actor.to_string());
    let policy = if request.uri().path().ends_with("/sell") {
        oteryn_web::ratelimit::NamedPolicy::MarketplaceListing
    } else {
        oteryn_web::ratelimit::NamedPolicy::MarketplaceBid
    };
    let lang = if request.uri().path().starts_with("/pl/") {
        oteryn_web::Language::Pl
    } else {
        oteryn_web::Language::En
    };
    match limiter.check(policy, &[keys]).await {
        Ok(()) => next.run(request).await,
        Err(denial) => denial.response(lang, false),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Control {
    request_id: Uuid,
    action: String,
    #[serde(rename = "_csrf")]
    _csrf: Option<String>,
}
async fn control(
    HttpState(runtime): HttpState<Runtime>,
    Path((lang, id)): Path<(String, Uuid)>,
    Extension(session): Extension<BrowserSession>,
    headers: HeaderMap,
    BrowserForm(input): BrowserForm<Control>,
) -> Response {
    if let Err(e) = locale(&lang) {
        return failure("en", e);
    }
    let Some(actor) = session.account_id().await else {
        return failure(&lang, Error::Denied);
    };
    let hash = match session_hash(&headers) {
        Ok(h) => h,
        Err(e) => return failure(&lang, e),
    };
    match runtime
        .control(actor, hash, id, input.request_id, &input.action)
        .await
    {
        Ok(()) => Redirect::to(&format!("/{lang}/bazaar/{id}")).into_response(),
        Err(e) => failure(&lang, e),
    }
}

#[derive(sqlx::FromRow)]
struct Candidate {
    #[sqlx(flatten)]
    row: Listing,
    created_at: OffsetDateTime,
}
impl std::ops::Deref for Candidate {
    type Target = Listing;
    fn deref(&self) -> &Listing {
        &self.row
    }
}
async fn candidate_page(
    db: &mut PgConnection,
    actor: Option<Uuid>,
    listing: Option<Uuid>,
    only_own: bool,
    cursor: Option<(OffsetDateTime, Uuid)>,
) -> Result<Vec<Candidate>, Error> {
    Ok(sqlx::query_as("SELECT l.* FROM bazaar_listings l JOIN bazaar_authority_assets a ON a.listing_id=l.id WHERE ($1::uuid IS NULL OR l.id=$1) AND (NOT $2 OR l.owner=$3) AND (l.state='active' OR l.owner=$3 OR EXISTS(SELECT 1 FROM bazaar_settlements s WHERE s.listing_id=l.id AND s.buyer=$3) OR EXISTS(SELECT 1 FROM bazaar_authority_gold_sales s WHERE s.listing_id=l.id AND s.buyer=$3)) AND ($4::timestamptz IS NULL OR (l.created_at,l.id)<($4,$5)) ORDER BY l.created_at DESC,l.id DESC LIMIT 50").bind(listing).bind(only_own).bind(actor).bind(cursor.map(|c|c.0)).bind(cursor.map(|c|c.1)).fetch_all(db).await?)
}
#[cfg(test)]
mod catalogue_tests {
    use super::*;
    #[tokio::test]
    async fn postgres_terminal_history_cannot_hide_active_catalogue_candidate()
    -> Result<(), Box<dyn std::error::Error>> {
        let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
        let started = std::time::Instant::now();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(&std::env::var("PORTAL_TEST_DATABASE_URL")?)
            .await?;
        oteryn_db::migrate(&pool).await?;
        let mut db = pool.acquire().await?;
        sqlx::raw_sql("CREATE TEMP TABLE bazaar_listings (LIKE public.bazaar_listings INCLUDING DEFAULTS); CREATE TEMP TABLE bazaar_authority_assets (LIKE public.bazaar_authority_assets INCLUDING DEFAULTS)").execute(&mut *db).await?;
        let active = Uuid::now_v7();
        for n in 0..101_i32 {
            let listing = if n == 0 { active } else { Uuid::now_v7() };
            sqlx::query("INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,state,ends_at,created_at) VALUES($1,$2,'item','auction','coins',100,false,CASE WHEN $3=0 THEN 'active'::bazaar_state ELSE 'completed'::bazaar_state END,now()+interval '1 day',now()+make_interval(secs=>$3))")
                .bind(listing).bind(Uuid::now_v7()).bind(n).execute(&mut *db).await?;
            sqlx::query("INSERT INTO bazaar_authority_assets(listing_id,request_id,world_id,seller_character_id,asset_id,expected_revision,session_hash,quantity,publication_consent,input_binding) VALUES($1,$1,$1,$1,$1,1,$2,1,true,'catalogue-test')").bind(listing).bind([1_u8;32].as_slice()).execute(&mut *db).await?;
        }
        let page = candidate_page(&mut db, None, None, false, None).await?;
        assert_eq!(page.len(), 1);
        assert_eq!(page[0].id, active);
        sqlx::query("UPDATE bazaar_listings SET state='active'")
            .execute(&mut *db)
            .await?;
        let page = candidate_page(&mut db, None, None, false, None).await?;
        assert_eq!(page.len(), 50);
        let last = &page[49];
        let second =
            candidate_page(&mut db, None, None, false, Some((last.created_at, last.id))).await?;
        assert_eq!(second.len(), 50);
        assert!(
            !second
                .iter()
                .any(|row| page.iter().any(|first| first.id == row.id))
        );
        let last = &second[49];
        let third =
            candidate_page(&mut db, None, None, false, Some((last.created_at, last.id))).await?;
        assert_eq!(third.len(), 1);
        assert_eq!(third[0].id, active);
        eprintln!(
            "postgres_terminal_history_cannot_hide_active_catalogue_candidate: {:?}",
            started.elapsed()
        );
        Ok(())
    }
}

#[cfg(test)]
#[tokio::test]
async fn r7_character_sale_finalizing_until_unlock() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    use crate::game::authority_tests::{BrowserFixture, uid};
    let db = BrowserFixture::new().await;
    sqlx::query("UPDATE bazaar_listings SET state='awaiting_delivery'")
        .execute(&db.pool)
        .await?;
    sqlx::query("UPDATE bazaar_listings SET state='completed',kind='character'")
        .execute(&db.pool)
        .await?;
    let (cards, _) = db
        .runtime
        .cards(Some(uid("11")), Some(uid("20")), true, None)
        .await?;
    assert_eq!(cards[0].state, "finalizing");
    db.end().await;
    Ok(())
}
