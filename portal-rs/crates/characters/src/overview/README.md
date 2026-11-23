# Account overview

The overview at `/account` resolves its principal from the browser session.
`OTERYN_ACCOUNT_OVERVIEW_ENABLED=true` enables it for active accounts. Private
responses are excluded from caching and indexing.

The localized account display editor at `/{lang}/account/display` edits the
existing account display name and preferred language. The server rechecks the
verified account, current session and closure state in the save transaction.
It uses the standard browser CSRF middleware and the existing Identity field
validation and privacy event policy.

Website login recency comes from successful login events. The roster uses a
fresh LCFA cut and shows the watermark in UTC. Game level and vocation are
read through the existing public profile flag and matched to the private roster.
The Game login history reader supplies the latest verified Game login if its
read controls and source are configured. Missing evidence displays Unavailable.

The wallet balance uses the existing independently flagged read.
The overview shows configured character limits as informational values. The
operator configuration and Game qualification requirements are listed in
[the GAP-10 product note](../../../../../docs/product/GAP-10_ACCOUNT_OVERVIEW.md).

The focused library tests use `PORTAL_TEST_DATABASE_URL` and isolated schemas.
