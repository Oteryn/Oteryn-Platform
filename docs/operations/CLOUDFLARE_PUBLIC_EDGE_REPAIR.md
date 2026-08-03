# Cloudflare Oteryn public-edge repair

## Purpose

The canonical public endpoints are:

```text
oteryn.molehill.cloud
gateway.molehill.cloud
```

Tunnel, DNS and Universal SSL are proven current. The edge repair preserves the existing broad country restriction for unrelated hostnames while exempting the two canonical hosts from controls that otherwise return a Cloudflare interstitial before requests reach Platform or Game Gateway.

## Exact repair contract

The canonical rule uses:

```json
{
  "action": "skip",
  "action_parameters": {
    "ruleset": "current",
    "products": ["bic", "securityLevel"]
  },
  "expression": "http.host in {\"oteryn.molehill.cloud\" \"gateway.molehill.cloud\"}"
}
```

The rule must be **the first rule** in the zone `http_request_firewall_custom` entry-point ruleset. A rule that merely appears before the country block is insufficient: an earlier matching rule with `ruleset: current` can skip all later custom rules before this rule's Browser Integrity Check and Security Level product exemptions execute.

Putting the exact-host rule first skips later custom rules, Browser Integrity Check and Security Level only for the two canonical hosts. The broad country restriction remains active for every unrelated hostname.

Bot Fight Mode is outside the Ruleset Engine and cannot be bypassed by a WAF skip rule. Because the public Game Gateway must accept native clients from arbitrary source addresses, Bot Fight Mode is disabled at zone scope. Other Bot Management fields are left unchanged.

Authoritative Cloudflare references:

- custom skip rule API examples: `https://developers.cloudflare.com/waf/custom-rules/skip/api-examples/`;
- available skip products and phases: `https://developers.cloudflare.com/waf/custom-rules/skip/options/`;
- rule ordering and first-position PATCH: `https://developers.cloudflare.com/ruleset-engine/rulesets-api/update-rule/`;
- Bot Fight Mode limitations: `https://developers.cloudflare.com/bots/get-started/bot-fight-mode/`;
- Bot Management update API: `https://developers.cloudflare.com/api/resources/bot_management/methods/update/`.

## Operational modes

The script supports:

```text
audit
apply
rollback
```

`audit` performs only GET requests. It reports whether the repair rule is exact, before the audited country block and at index zero. An exact rule that is not first is classified as `shadowed_by_earlier_rules` rather than accepted as desired state.

`apply` requires:

```text
APPLY-OTERYN-PUBLIC-EDGE-REPAIR
```

It fails closed unless there is exactly one audited broad country-block candidate and zero or one exact Oteryn repair rule. It:

1. creates an absent rule at the top, or moves the existing exact rule to the top using a zone-ruleset rule PATCH;
2. disables Bot Fight Mode when still enabled;
3. re-reads the ruleset and Bot state;
4. requires the exact rule at index zero, before the country block, and Bot Fight Mode off.

A partial failure automatically restores Bot Fight Mode. A newly created rule is deleted. A pre-existing rule moved during the failed attempt is restored to its previous position before the original successor rule.

`rollback` requires:

```text
ROLLBACK-OTERYN-PUBLIC-EDGE-REPAIR
```

It accepts only the exact managed rule reference and definition. The rule description records whether Bot Fight Mode was on or off before the original apply, allowing rollback to restore that baseline before deleting the rule.

## Trusted execution boundary

Pull requests run deterministic mock API tests without Cloudflare credentials. Live operations execute only after an exact marker reaches `main` and the job enters the protected GitHub environment `production-cloudflare`.

The workflow uses:

```text
CLOUDFLARE_EDGE_AUDIT_TOKEN
```

with zone-bounded `Zone WAF Edit` and `Bot Management Edit`. The account token remains separate and is used only by the post-operation Access audit.

Operational marker forms are exact and marker-only for `audit`, `apply` and `rollback`. An `inert` marker may accompany implementation cleanup and performs no Cloudflare request.

## Evidence and E2E

After `apply` or `rollback`, the workflow runs the read-only Cloudflare edge collector and independent public DNS/TLS/HTTP validator. It uploads sanitized evidence for 14 days and publishes a fixed allowlist result to Issue #91.

The published result excludes:

- tokens and authorization headers;
- full API responses;
- WAF expressions and country literals;
- response bodies and cookies;
- unrelated hostnames and private origins.

A product-level public `FAIL` is retained as evidence. API or collector execution failure fails the workflow. The repair script automatically rolls back its own partial mutation before returning an apply failure.
