# Staging commerce coverage

Source reviewed at `7847cf7de91957d4317e7693d31d049b8f1c5f8e`. The public router starts in [main.rs](../../../../bin/oteryn-portal/src/main.rs#L249). It merges the payments account and native-coin routers at lines 297–298, Bazaar at lines 344–347 and 384–388, Store at line 374, moderation at lines 465–468, and codes at line 515. Lines 517–546 split the public and game listeners. [game_ingress.rs](../../../../bin/oteryn-portal/src/game_ingress.rs#L75) mounts Bazaar Game ingress on a separate private listener.

Paths below use `{lang}` for Polish or English. Inputs include query, path and body fields where applicable. Feature gates describe the enabled composition. Browser POST forms use the portal session and `_csrf`, except signed provider and service protocols. Staff routes also use the [admin host, session and MFA guard](../../../../crates/admin/src/web.rs#L206).

## Store and payments

| Method and path | Enabled access and gate | Inputs |
| --- | --- | --- |
| GET, POST `/{lang}/shop` | Signed-in Store buyer. `OTERYN_SHOP_ENABLED`. The configured browser composition disables purchases without authoritative Game ownership. | POST `_csrf, operation, character, world, revision, offer, price, quantity`. |
| GET, POST `/{lang}/shop/coins` | Signed-in Store user. Shop flag. POST also requires the top-up adapter, `OTERYN_SHOP_TOP_UPS_ENABLED`, wallet and payments. | POST `_csrf, operation, package`. |
| GET `/{lang}/shop/history` | Signed-in account, shop flag. | Query `before`. |
| GET, POST `/{lang}/shop/transfer` | Signed-in sender, shop flag, testing mode, `OTERYN_SHOP_TEST_PAYMENTS_ENABLED`, `OTERYN_SHOP_TRANSFERS_ENABLED` and transfer eligibility. | POST `_csrf, operation, recipient, amount`. |
| GET, POST `/{lang}/shop/test-checkout/{intent}` | Intent owner, shop and top-up flags and testing provider. | Intent UUID. POST `_csrf, action`. |
| GET, POST `/{lang}/admin/store/codes` | Staff with `store.codes.generate`. `OTERYN_STORE_CODES_ENABLED`. | POST `_csrf, request_id, kind, amount, count, max_per_code, max_per_account, valid_from, valid_until, note`. |
| POST `/{lang}/shop/codes/redeem` | Signed-in eligible redeemer. Codes flag and wallet or premium service, according to code kind. | JSON `code, operation`, session CSRF header. |
| GET `/{lang}/panel/premium` | Signed-in account, `PANEL_PREMIUM_PAYMENTS`. | None. |
| GET `/{lang}/panel/payments` | Signed-in account, same panel flag. | Query `page`. |
| GET `/{lang}/panel/payments/receipts/{id}` | Signed-in receipt owner, same panel flag. | Receipt UUID. |
| GET `/api/v1/native-account/coin-purchases/catalogue` | Native OAuth `game:account-services`. `PORTAL_COIN_PURCHASES_ENABLED` and testing or preprod. | Optional `character_id` query. |
| GET `/api/v1/native-account/coin-purchases/balance` | Native OAuth, same coin-purchase gate. | No query or form fields. |
| POST `/api/v1/native-account/coin-purchases/orders` | Native OAuth payer, same coin-purchase gate. | JSON `version, request_id, package_id, quote_id, character_id` (nullable). |
| GET `/api/v1/native-account/coin-purchases/requests/{request_id}` | Native OAuth payer who submitted the request, same coin-purchase gate. | Request UUIDv7. |
| GET, POST `/native-account/coin-checkout/{order_id}` | Browser session for payer, same coin-purchase gate. | Order UUIDv7. POST `_csrf, action` (`pay` or `fail`). |
| POST `/api/v1/payments/provider-webhook` | Signed provider requests, configured signing secret, `PORTAL_PAYMENTS_PROVIDER_WEBHOOK_ENABLED` and coin-purchase service. | Envelope `version, event_id, type`. Reversal adds `order_reference, amount`. |
| POST `/__test/payments/webhook` | Testing mode, `OTERYN_SHOP_TEST_PAYMENTS_ENABLED`, test-provider signature and configured payments service. | Signed JSON payment event. See `provider.rs`. |

Source files include [Store browser and forms](../../../../crates/store/src/web.rs#L74), [Store checkout](../../../../crates/store/src/web_checkout.rs#L17), [testing payments composition](../../../../crates/store/src/web_payments.rs#L276), [codes](../../../../bin/oteryn-portal/src/code_routes.rs#L114), [account payments](../../../../crates/payments/src/account.rs#L34), [native payments](../../../../crates/payments/src/coin_http.rs#L30), [provider webhook](../../../../crates/payments/src/provider_webhook.rs#L150).

## Bazaar and staff commerce

| Method and path | Enabled access and gate | Inputs |
| --- | --- | --- |
| GET `/{lang}/bazaar` and `/{lang}/bazaar/items` | Public catalogue. Real listing data needs the qualified `OTERYN_BAZAAR_ENABLED` Game runtime. | Runtime query `before`. |
| GET `/{lang}/account/bazaar` | Signed-in account, Game runtime. | None. |
| GET `/{lang}/bazaar/{listing}` and `/{lang}/bazaar/items/{listing}` | Public listing details. Owner controls and bidder forms depend on actor and state. | Listing UUID. |
| GET `/{lang}/account/bazaar/sell`, `/{lang}/bazaar/items/new` | Signed-in eligible seller, Game runtime. | None. |
| POST `/{lang}/account/bazaar/sell` | Signed-in eligible owner of the Game asset, Game runtime. | Runtime `_csrf, request_id, currency, kind, world_id, seller_character_id, asset_id, expected_revision, quantity, definition_key, definition_revision, starting_bid, duration_days, publish_snapshot`. Character-only adapter `_csrf, character_id, currency, starting_bid, duration_days, request_id, publish_snapshot`. |
| POST `/{lang}/account/bazaar/{listing}/bids` | Signed-in eligible buyer, external to the listing, Game runtime. | Runtime `_csrf, request_id, maximum, buyer_character_id`. Character-only adapter `_csrf, request_id, amount`. |
| POST `/{lang}/account/bazaar/{listing}/control` | Signed-in listing owner, Game runtime. | `_csrf, request_id, action` (`resume` or `cancel`). |
| POST `/{lang}/bazaar/items` | Item compatibility adapter. A qualified item-authority composition requires an eligible signed-in seller. The fallback always returns 503. | `_csrf, request_id, mode, definition_key, definition_revision, world_id, seller_character_id, item_instance_id, quantity, unit_price, buy_now_unit_price, auction_days, message`. |
| POST `/{lang}/bazaar/items/{id}/{action}` | Item compatibility adapter. `bid, offer, buy-now` require eligible buyers. `accept, reject, publish, cancel, renew` require owner authority. Fallback 503. | `_csrf, request_id`, `amount` for bid or offer, `offer_id` for accept or reject. |
| GET `/{lang}/bazaar/sellers/{seller}` | Public with `OTERYN_BAZAAR_MODERATION` and `OTERYN_BAZAAR_PROFILES`. | Seller UUID. |
| GET, POST `/{lang}/account/bazaar/{listing}/report` | Signed-in reporter, `OTERYN_BAZAAR_MODERATION`. | Listing UUID. POST `_csrf, id, category, detail`. |
| GET, POST `/{lang}/account/bazaar/{listing}/promote` | Listing owner, moderation and `OTERYN_BAZAAR_PROMOTIONS`. | Listing UUID. POST `_csrf, id, fee_revision, amount`. |
| GET, POST `/admin/{lang}/bazaar` | Staff `marketplace.manage`, moderation flag. | POST `_csrf, action, id, listing, reason, coin_bps, gold_bps, listing_fee, promotion_price, world, family, production_key, revision_ref, minimum_value, accepted` according to action. |
| GET `/admin/commerce` | Staff with at least one of `products.premium.manage`, `marketplace.manage`, `payments.reconcile`. `OTERYN_ADMIN_COMMERCE`. | Query `after`. |
| POST `/admin/commerce/{action}` | Staff and commerce flag. Grant/revoke need premium permission and premium flag. Adjust needs marketplace permission and wallet flag. Reconcile/refund need payments permission and payments flag. Refund also needs refunds enabled. | `_csrf, request_id, target, reason`, `days` for grant, `amount` for adjust. |
| POST `/admin/commerce/approvals/{id}` and `/admin/commerce/cancellations/{id}` | Commerce staff authorized for the underlying action. Maker/checker and intent-state constraints apply. | Approval UUID, `_csrf`. |

Source files include [character fallback and forms](../../../../crates/bazaar/src/characters/http.rs#L97), [Game browser router and forms](../../../../crates/bazaar/src/game/runtime/browser.rs#L187), [Game listing fields](../../../../crates/bazaar/src/game/runtime/mod.rs#L481), [item adapter](../../../../crates/bazaar/src/items/web.rs#L71), [moderation](../../../../crates/bazaar/src/moderation_web.rs#L60), [staff Bazaar](../../../../crates/admin/src/bazaar.rs#L138), [staff commerce](../../../../crates/admin/src/commerce/http.rs#L52), [built-in role grants](../../../../crates/admin/src/rbac/catalog.rs#L315).

Platform Administrator is the applicable built-in role for code issuance, premium and payments. Moderator shares marketplace-authorized Bazaar and wallet adjustments. Support supplies the staff role without any of these permissions. The suite uses the named role-specific staging accounts.

## Private commerce and Game routes

The HTTP checks use the public staging host, where private-listener paths should be absent. Browser cookies, staff roles and native browser locale do not supply the Game identity.

| Method and path | Private access and gate | Inputs |
| --- | --- | --- |
| POST `/internal/v1/coins/transfers` | Coin mTLS identity and `PORTAL_COIN_TRANSFERS_ENABLED`. | `operation_id, sender_account_id, recipient_account_id, amount`. |
| POST `/internal/v1/coins/market/offers` | Coin mTLS identity and `PORTAL_COIN_MARKET_ENABLED`. | `operation_id, seller_account_id, amount, gold_price`. |
| POST `/internal/v1/coins/market/settlements` | Coin mTLS identity and market flag. | `operation_id, reservation_id, buyer_account_id, amount`. |
| POST `/internal/v1/coins/market/cancellations` | Coin mTLS identity and market flag. | `operation_id, seller_account_id, reservation_id`. |
| GET `/internal/v1/coins/operations/{operation_id}` | Coin mTLS identity and transfer or market flag. | Operation UUID. |
| GET `/internal/v1/coins/accounts/{account_id}/balance` | Coin mTLS identity and transfer or market flag. | Account UUID. |
| GET `/internal/v1/coins/accounts/{account_id}/history` | Coin mTLS identity and transfer or market flag. | Account UUID, query `cursor, limit`. |
| POST `/internal/v1/products-entitlements/store/balance/read`, `history/read`, `purchases` | Store mTLS identity, `PRODUCTS_ENTITLEMENTS_STORE_DELIVERY_ENABLED` and `PRODUCTS_ENTITLEMENTS_STORE_PURCHASE_ENABLED`, valid authority configuration. | Store JSON head `contract_version, operation, source_authority`. Balance adds `account_id`. History adds `account_id, before_entry_id, max_entries`. Purchase adds `purchase_request_id, account_id, character_id, world_id, catalog_revision, offer_key, currency, unit_price, quantity`. |
| POST `/internal/v1/products-entitlements/store/operations/pending`, `read`, `receipt`, `reconcile` | Store mTLS identity, delivery flag and valid authority configuration. | Store JSON head. Pending/reconcile add `after_source_revision, after_operation_id, max_entries`. Read adds `issuer_authority, operation_kind, operation_id`. Receipt adds the binding, outcome and operation-specific fields in contract §5.6. |
| POST `/internal/v1/products-entitlements/premium-snapshots/read` | Premium snapshot mTLS identity and `PRODUCTS_ENTITLEMENTS_PREMIUM_SNAPSHOT_ENABLED`. | JSON `schema, account_id, nonce`. |
| POST `/internal/v1/products-entitlements/loyalty-snapshots/read` | Loyalty snapshot mTLS identity and `PRODUCTS_ENTITLEMENTS_LOYALTY_SNAPSHOT_ENABLED`. | JSON `schema, account_id, nonce`. |
| POST `/internal/v1/products-entitlements/loyalty-snapshots/stream` | Loyalty stream mTLS identity and `PRODUCTS_ENTITLEMENTS_LOYALTY_STREAM_ENABLED`. | JSON `schema, after, limit`. |
| POST `/internal/v1/products-entitlements/profile-b/authority/read` | Profile B authority mTLS identity and `PRODUCTS_ENTITLEMENTS_PROFILE_B_AUTHORITY_ENABLED`. | JSON `schema, account_id, nonce`. |
| POST `/internal/v1/products-entitlements/profile-b/changes/read` | Profile B changes mTLS identity and `PRODUCTS_ENTITLEMENTS_PROFILE_B_CHANGES_ENABLED`. | JSON `schema, after, limit`. |
| POST `/internal/v1/bazaar/commands/pending`, `read`, `reconcile` | Bazaar mTLS identity and qualified Game runtime. | Bounded JSON command exchange, see `authority_exchange.rs`. |
| POST `/internal/v1/bazaar/transferability`, `status`, `watermark` | Bazaar mTLS identity and qualified Game runtime. | Bounded JSON transferability, status and watermark publications, see `authority_http.rs`. |
| POST `/internal/v1/game-auth/character-commands/receipt` | Character receipt mTLS identity and qualified runtime for Bazaar version 2 receipts. | Versioned command receipt, see `authority_wire.rs`. |
| POST `/internal/v1/game-events` | Verified event-ingress TLS peer and qualified Game runtime. | Bounded `application/x-protobuf` event envelope, see `event_wire.rs` and `game_wire.rs`. |

Source files include [wallet wire](../../../../crates/wallet/src/wire.rs#L38), [Store wire contract](../../../../../docs/contracts/OTERYN_V2_STORE_WIRE_CONTRACT.md#5-messages), [Store private mounts](../../../../bin/oteryn-portal/src/game_routes.rs), [Bazaar private exchange and receipts](../../../../crates/bazaar/src/game/authority_http.rs#L30), [Bazaar event ingress](../../../../crates/bazaar/src/game/runtime/transport.rs#L25).

## Staging outcomes by route and actor

Each row covers the four projects. The anonymous outcome for an account or panel GET reflects the [shared chrome guard](../../../../crates/web/src/chrome.rs#L28), which runs before the module handler. Bazaar 503 checks assert the exact locale message from the [character copy](../../../../templates/bazaar/en.ftl#L70) or the [item copy](../../../../crates/bazaar/src/items/en.ftl#L2). POST coverage uses empty requests without CSRF tokens, repeated empty submissions and inert malformed/oversize fields. Public refusals for private-listener routes use a minimal request.

A and S in the test column refer to tests named `<Module> disabled for anonymous visitors` and `commerce disabled for staff <role>`. P refers to `commerce disabled for two fresh players`. The signed-in columns are expected status / observed outcome. The player tests use two fresh identities. The first would own the commerce record and the second would exercise the other-party view. Ownership itself needs a commerce record that staging can create.

| Method and path | Anonymous expected / observed | Player 1 expected / observed | Player 2 expected / observed | Platform Admin expected / observed | Moderator expected / observed | Support expected / observed | Tests |
| --- | --- | --- | --- | --- | --- | --- | --- |
| GET `/{lang}/shop` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Store A, P, S |
| POST `/{lang}/shop` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Store A, P, S |
| GET `/{lang}/shop/coins` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Store A, P, S |
| POST `/{lang}/shop/coins` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Store A, P, S |
| GET `/{lang}/shop/history` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Store A, P, S |
| GET `/{lang}/shop/transfer` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Store A, P, S |
| POST `/{lang}/shop/transfer` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Store A, P, S |
| GET `/{lang}/shop/test-checkout/{id}` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Store A, P, S |
| POST `/{lang}/shop/test-checkout/{id}` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Store A, P, S |
| GET `/{lang}/admin/store/codes` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Store A, P, S |
| POST `/{lang}/admin/store/codes` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Store A, P, S |
| POST `/{lang}/shop/codes/redeem` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Store A, P, S |
| POST `/internal/v1/products-entitlements/store/balance/read` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Store A, P, S |
| POST `/internal/v1/products-entitlements/store/history/read` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Store A, P, S |
| POST `/internal/v1/products-entitlements/store/purchases` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Store A, P, S |
| POST `/internal/v1/products-entitlements/store/operations/pending` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Store A, P, S |
| POST `/internal/v1/products-entitlements/store/operations/read` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Store A, P, S |
| POST `/internal/v1/products-entitlements/store/operations/receipt` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Store A, P, S |
| POST `/internal/v1/products-entitlements/store/operations/reconcile` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Store A, P, S |
| POST `/internal/v1/products-entitlements/premium-snapshots/read` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Store A, P, S |
| POST `/internal/v1/products-entitlements/loyalty-snapshots/read` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Store A, P, S |
| POST `/internal/v1/products-entitlements/loyalty-snapshots/stream` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Store A, P, S |
| POST `/internal/v1/products-entitlements/profile-b/authority/read` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Store A, P, S |
| POST `/internal/v1/products-entitlements/profile-b/changes/read` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Store A, P, S |
| POST `/internal/v1/coins/transfers` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Wallet A, P, S |
| POST `/internal/v1/coins/market/offers` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Wallet A, P, S |
| POST `/internal/v1/coins/market/settlements` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Wallet A, P, S |
| POST `/internal/v1/coins/market/cancellations` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Wallet A, P, S |
| GET `/internal/v1/coins/operations/{id}` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Wallet A, P, S |
| GET `/internal/v1/coins/accounts/{id}/balance` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Wallet A, P, S |
| GET `/internal/v1/coins/accounts/{id}/history` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Wallet A, P, S |
| GET `/{lang}/panel/premium` | 303 / 303 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Payments A, P, S |
| GET `/{lang}/panel/payments` | 303 / 303 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Payments A, P, S |
| GET `/{lang}/panel/payments/receipts/{id}` | 303 / 303 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Payments A, P, S |
| GET `/api/v1/native-account/coin-purchases/catalogue` | 503 / 503 | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | Payments A, P, S |
| GET `/api/v1/native-account/coin-purchases/balance` | 503 / 503 | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | Payments A, P, S |
| POST `/api/v1/native-account/coin-purchases/orders` | 503 / 503 | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | Payments A, P, S |
| GET `/api/v1/native-account/coin-purchases/requests/{id}` | 503 / 503 | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | Payments A, P, S |
| GET `/native-account/coin-checkout/{id}` | 503 / 503 | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | Payments A, P, S |
| POST `/native-account/coin-checkout/{id}` | 503 / 503 | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | Payments A, P, S |
| POST `/api/v1/payments/provider-webhook` | 503 / 503 | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | Payments A, P, S |
| POST `/__test/payments/webhook` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Payments A, P, S |
| GET `/{lang}/bazaar` | 503 / 503 | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | Bazaar A, P, S |
| GET `/{lang}/account/bazaar` | 303 / 303 | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | Bazaar A, P, S |
| GET `/{lang}/bazaar/{id}` | 503 / 503 | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | Bazaar A, P, S |
| GET `/{lang}/account/bazaar/sell` | 303 / 303 | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | Bazaar A, P, S |
| POST `/{lang}/account/bazaar/sell` | 405 / 405 | 405 / Blocked | 405 / Blocked | 405 / Blocked | 405 / Blocked | 405 / Blocked | Bazaar A, P, S |
| POST `/{lang}/account/bazaar/{id}/bids` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Bazaar A, P, S |
| POST `/{lang}/account/bazaar/{id}/control` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Bazaar A, P, S |
| GET `/{lang}/bazaar/items` | 503 / 503 | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | Bazaar A, P, S |
| POST `/{lang}/bazaar/items` | 503 / 503 | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | Bazaar A, P, S |
| GET `/{lang}/bazaar/items/new` | 503 / 503 | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | Bazaar A, P, S |
| GET `/{lang}/bazaar/items/{id}` | 503 / 503 | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | Bazaar A, P, S |
| POST `/{lang}/bazaar/items/{id}/bid` | 503 / 503 | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | Bazaar A, P, S |
| POST `/{lang}/bazaar/items/{id}/offer` | 503 / 503 | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | Bazaar A, P, S |
| POST `/{lang}/bazaar/items/{id}/accept` | 503 / 503 | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | Bazaar A, P, S |
| POST `/{lang}/bazaar/items/{id}/reject` | 503 / 503 | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | Bazaar A, P, S |
| POST `/{lang}/bazaar/items/{id}/buy-now` | 503 / 503 | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | Bazaar A, P, S |
| POST `/{lang}/bazaar/items/{id}/publish` | 503 / 503 | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | Bazaar A, P, S |
| POST `/{lang}/bazaar/items/{id}/cancel` | 503 / 503 | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | Bazaar A, P, S |
| POST `/{lang}/bazaar/items/{id}/renew` | 503 / 503 | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | 503 / Blocked | Bazaar A, P, S |
| GET `/{lang}/bazaar/sellers/{id}` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Bazaar A, P, S |
| GET `/{lang}/account/bazaar/{id}/report` | 303 / 303 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Bazaar A, P, S |
| POST `/{lang}/account/bazaar/{id}/report` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Bazaar A, P, S |
| GET `/{lang}/account/bazaar/{id}/promote` | 303 / 303 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Bazaar A, P, S |
| POST `/{lang}/account/bazaar/{id}/promote` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Bazaar A, P, S |
| GET `/admin/{lang}/bazaar` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Bazaar A, P, S |
| POST `/admin/{lang}/bazaar` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Bazaar A, P, S |
| POST `/internal/v1/bazaar/commands/pending` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Bazaar A, P, S |
| POST `/internal/v1/bazaar/commands/read` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Bazaar A, P, S |
| POST `/internal/v1/bazaar/commands/reconcile` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Bazaar A, P, S |
| POST `/internal/v1/bazaar/transferability` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Bazaar A, P, S |
| POST `/internal/v1/bazaar/status` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Bazaar A, P, S |
| POST `/internal/v1/bazaar/watermark` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Bazaar A, P, S |
| POST `/internal/v1/game-auth/character-commands/receipt` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Bazaar A, P, S |
| POST `/internal/v1/game-events` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Bazaar A, P, S |
| GET `/admin/commerce` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Staff commerce A, P, S |
| POST `/admin/commerce/grant` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Staff commerce A, P, S |
| POST `/admin/commerce/revoke` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Staff commerce A, P, S |
| POST `/admin/commerce/adjust` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Staff commerce A, P, S |
| POST `/admin/commerce/reconcile` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Staff commerce A, P, S |
| POST `/admin/commerce/refund` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Staff commerce A, P, S |
| POST `/admin/commerce/approvals/{id}` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Staff commerce A, P, S |
| POST `/admin/commerce/cancellations/{id}` | 404 / 404 | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | 404 / Blocked | Staff commerce A, P, S |
