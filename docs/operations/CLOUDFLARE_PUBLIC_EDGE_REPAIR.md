# Cloudflare Oteryn public-edge repair

## Purpose

The canonical public endpoints are:

```text
oteryn.molehill.cloud
gateway.molehill.cloud
```

Tunnel, DNS and Universal SSL are already proven current. The remaining edge failure is a Cloudflare policy interstitial before requests reach Platform or Game Gateway.

This repair preserves the existing broad country restriction for unrelated hostnames. It adds one earlier exact-host custom WAF `skip` rule for the two canonical hosts and disables Bot Fight Mode.

## Why both controls are required

The exact-host rule uses:

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

This skips later custom rules, Browser Integrity Check and Security Level only for the canonical Oteryn hosts. It does not disable the country rule for any other service.

Cloudflare Bot Fight Mode is outside the Ruleset Engine and cannot be bypassed by a WAF skip rule. Because the public Game Gateway must accept native clients from arbitrary source addresses, Bot Fight Mode is disabled at zone scope. Other Bot Management fields are left unchanged.

Authoritative Cloudflare references:

- custom skip rule API examples: `https://developers.cloudflare.com/waf/custom-rules/skip/api-examples/`;
- available skip products and phases: `https://developers.cloudflare.com/waf/custom-rules/skip/options/`;
- Bot Fight Mode limitations: `https://developers.cloudflare.com/bots/get-started/bot-fight-mode/`;
- Bot Management update API: `https://developers.cloudflare.com/api/resources/bot_management/methods/update/`.

## Operational modes

The script supports:

```text
audit
apply
rollback
```

`audit` performs only GET requests.

`apply` requires:

```text
APPLY-OTERYN-PUBLIC-EDGE-REPAIR
```

It fails closed unless there is exactly one broad enabled country-block candidate and zero or one exact Oteryn repair rule. It creates the rule immediately before the block candidate, disables Bot Fight Mode, re-reads both controls and verifies the exact desired state. A partial failure automatically restores Bot Fight Mode and deletes a newly created rule.

`rollback` requires:

```text
ROLLBACK-OTERYN-PUBLIC-EDGE-REPAIR
```

It accepts only the exact managed rule reference and definition. The rule description records whether Bot Fight Mode was on or off before apply, allowing rollback to restore that baseline before deleting the rule.

## Trusted execution boundary

Pull requests run deterministic mock API tests without Cloudflare credentials. Live operations execute only after an exact marker reaches `main` and the job enters the protected GitHub environment `production-cloudflare`.

The workflow uses:

```text
CLOUDFLARE_EDGE_AUDIT_TOKEN
```

with zone-bounded `Zone WAF Edit` and `Bot Management Edit`. The existing account token remains separate and is used only by the post-operation Access audit.

Operational marker forms are exact and marker-only for `audit`, `apply` and `rollback`. An `inert` marker may accompany implementation cleanup and performs no Cloudflare request.

## Evidence and E2E

After `apply` or `rollback`, the workflow runs the existing read-only Cloudflare edge collector and independent public DNS/TLS/HTTP validator. It uploads sanitized evidence for 14 days and publishes a fixed allowlist result to Issue #91.

The published result excludes:

- tokens and authorization headers;
- full API responses;
- WAF expressions and country literals;
- response bodies and cookies;
- unrelated hostnames and private origins.

A product-level public `FAIL` is reported as evidence. API or collector execution failure fails the workflow. The repair script itself automatically rolls back any partial mutation before returning an apply failure.
