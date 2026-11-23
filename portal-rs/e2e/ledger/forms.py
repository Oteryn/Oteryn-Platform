"""Inventory HTML forms in Askama templates using only the standard library."""

import argparse
import json
import re
from html.parser import HTMLParser
from pathlib import Path

LEDGER = Path(__file__).resolve().parent
PORTAL = LEDGER.parent.parent

# Shared templates and implicit actions derive their destinations from Rust
# renderers. Keep the concrete alternatives for each form here.
ACTIONS = {
    "crates/admin/src/commerce/panels/catalog.html": [
        ("/admin/store-activation",), ("/admin/store-catalog",),
    ],
    "crates/admin/src/commerce/panels/payments.html": [("/admin/payment-review",)] * 2,
    "crates/admin/src/operations/page.html": [("/admin/operations/statistics", "/admin/operations/rate-limits")],
    "crates/admin/templates/support_queue.html": [("/admin/support/tickets",)],
    "bin/oteryn-portal/templates/code_issuance.html": [
        ("/{lang}/admin/store/codes",),
    ],
    "crates/admin/src/console/templates/page.html": [
        ("/console/dashboard", "/console/worlds", "/console/nodes", "/console/database"),
    ] * 2,
    "crates/admin/src/console/templates/read.html": [
        ("/admin/console/players",), ("/admin/console/players",), ("/admin/console/audit",),
        ("/admin/console/players/retry",),
    ],
    "crates/admin/src/console/enforcement/templates/index.html": [
        ("/admin/console/enforcement",), ("/admin/console/enforcement/step-up",),
        ("/admin/console/enforcement",),
    ],
    "crates/payments/templates/coin/checkout.html": [
        ("/native-account/coin-checkout/{order_id}",),
    ] * 2,
    "crates/support/src/tickets/page.html": [
        ("/{lang}/support/tickets",),
        ("/{lang}/support/tickets/{id}/read",),
        ("/{lang}/support/tickets/{id}/reply", "/{lang}/admin/support/tickets/{id}/reply"),
        ("/{lang}/support/tickets/{id}/status", "/{lang}/admin/support/tickets/{id}/status"),
    ],
    "templates/account/lifecycle.html": [
        ("/{lang}/account/privacy/delete", "/{lang}/account/privacy/delete/cancel",
         "/{lang}/account/notifications", "/{lang}/unsubscribe"),
    ],
    "templates/admin/analytics.html": [
        tuple("/admin/analytics" + suffix for suffix in ("", "/activity", "/creatures", "/levels", "/health")),
    ],
    "templates/admin/analytics_economy.html": [("/admin/analytics/economy",)],
    "templates/admin/events/form.html": [("/admin/events", "/admin/events/{id}"), ()],
    "templates/admin/pages/edit.html": [("/admin/pages/{slug}/{lang}",)],
    "templates/admin/pages/index.html": [("/admin/pages",)],
    "templates/admin/wiki/action.html": [
        tuple("/admin/wiki/articles/{id}/" + action
              for action in ("submit-review", "return-draft", "publish", "unpublish", "archive")),
    ],
    "templates/badges/selection.html": [("/account/badges",)],
    "templates/bazaar/characters.html": [
        ("/{lang}/account/bazaar/sell",), ("/{lang}/bazaar/{listing}",), (),
    ],
    "templates/bazaar/item.html": [
        tuple("/{lang}/bazaar/items/{id}/" + action for action in ("accept", "reject")),
        tuple("/{lang}/bazaar/items/{id}/" + action for action in ("publish", "renew", "cancel")),
        (), (),
    ],
    "templates/bestiary/page.html": [("/{lang}/atlas/bestiary/{character}",)],
    "templates/content/announcements.html": [
        ("/admin/announcements", "/admin/announcements/{id}/edit"),
    ],
    "templates/deaths/index.html": [
        ("/deaths", "/{lang}/deaths", "/characters/{name}/deaths", "/{lang}/characters/{name}/deaths"),
    ],
    "templates/guilds/page.html": [("/guilds", "/{lang}/guilds")],
    "templates/pages/online.html": [("/online", "/{lang}/online")],
    "templates/pages/rankings.html": [("/{lang}/rankings/{kind}",)],
    "templates/profile/settings.html": [("/account/characters/{name}/profile", "/{lang}/account/characters/{name}/profile")],
    "templates/search/index.html": [(), ("/search", "/{lang}/search")],
    "templates/stats/character/index.html": [("/{lang}/stats/characters/{id}/{tab}", "/{lang}/stats/compare")],
    "templates/stats/guilds/index.html": [("/stats/guilds", "/{lang}/stats/guilds")],
    "templates/stats/rankings/index.html": [("/{lang}/stats/rankings/{kind}",)],
    "templates/stats/world/index.html": [("/stats/world", "/{lang}/stats/world")],
    "templates/wiki/calculators/page.html": [("/{lang}/wiki/calculators",)] * 4,
    "templates/wiki/entities_list.html": [("/wiki/{kind}", "/{lang}/wiki/{kind}")] * 2,
    "templates/wiki/index.html": [(), ("/wiki", "/{lang}/wiki", "/wiki/search", "/{lang}/wiki/search")],
}

ASKAMA = re.compile(r"{{(?:(?!{{|}}).)*}}|{%(?:(?!{%|%}).)*%}|{#(?:(?!{#|#}).)*#}", re.S)
PARAM = re.compile(r"{([^{}]+)}")

CALCULATOR = "templates/wiki/calculators/page.html"

# These Rust renderers build forms at runtime. Their write registrations
# contribute route evidence, while the mutation inventory retains their markup.
ROUTE_ONLY_RENDERERS = {
    "crates/admin/src/bazaar.rs",
    "crates/admin/src/control/controls_http.rs",
    "crates/admin/templates/support_record.html",
    "crates/bazaar/src/game/runtime/browser.rs",
    "crates/bazaar/src/moderation_web.rs",
    "crates/characters/src/create_web/page.rs",
    "crates/community/src/party/party_view.rs",
    "crates/community/src/referrals/web.rs",
    "crates/content/src/news.rs",
    "crates/identity/src/consents_http.rs",
    "crates/identity/src/credentials_http.rs",
    "crates/identity/src/email_change_http.rs",
    "crates/identity/src/export_http.rs",
    "crates/identity/src/privacy_web.rs",
    "crates/identity/src/recovery_key_http.rs",
    "crates/identity/src/sessions_http.rs",
    "crates/minigames/src/collection_pvp/http.rs",
    "crates/minigames/src/puzzles/http.rs",
    "crates/playerdata/src/places/http.rs",
    "crates/playerdata/src/reports/render.rs",
    "crates/store/src/web.rs",
    "crates/support/src/reports/web.rs",
    "crates/web/src/routes/verify.rs",
}


class Forms(HTMLParser):
    def __init__(self, source):
        super().__init__(convert_charrefs=True)
        self.forms = []
        self.current = None
        self.expressions = []

        # Protect quoted Rust expressions inside attributes while retaining line numbers.
        def mask(match):
            index = len(self.expressions)
            self.expressions.append(match[0])
            return f"__ASKAMA_{index}__" + "\n" * match[0].count("\n")

        self.feed(ASKAMA.sub(mask, source))
        self.close()

    def restore(self, value):
        return re.sub(r"__ASKAMA_(\d+)__", lambda m: self.expressions[int(m[1])], value)

    def handle_starttag(self, tag, attributes):
        attrs = {key: self.restore(value or "") for key, value in attributes}
        if tag == "form":
            self.current = dict(line=self.getpos()[0], action=attrs.get("action", ""),
                                method=attrs.get("method", "GET").upper(), fields=set(), buttons=[], hidden={})
            self.forms.append(self.current)
        elif self.current is not None:
            kind = attrs.get("type", "submit" if tag == "button" else "").lower()
            if tag in {"button", "input"} and kind in {"submit", "image"}:
                button = None
                if attrs.get("name"):
                    button = dict(name=attrs["name"], value=attrs.get("value", ""))
                self.current["buttons"].append((attrs.get("formaction"), attrs.get("formmethod"), button))
            elif tag in {"input", "select", "textarea"} and attrs.get("name"):
                self.current["fields"].add(attrs["name"])
                if tag == "input" and kind == "hidden":
                    self.current["hidden"][attrs["name"]] = attrs.get("value", "")

    def handle_endtag(self, tag):
        if tag == "form":
            self.current = None


def expand(action):
    if action in ("/{{ lang }}{{ login_path }}", "/{{ lang }}{{ self.login_path() }}"):
        yield "/{lang}/login"
        return
    opening = alternative = None
    for token in ASKAMA.finditer(action):
        if not token[0].startswith("{%"):
            continue
        directive = token[0][2:-2].strip()
        if opening is None:
            if re.match(r"if\b", directive):
                opening = token
        elif directive == "else" and alternative is None:
            alternative = token
        elif directive == "endif":
            positive = action[opening.end():alternative.start() if alternative else token.start()]
            negative = action[alternative.end():token.start()] if alternative else ""
            for branch in (positive, negative):
                yield from expand(action[:opening.start()] + branch + action[token.end():])
            return

    def parameter(token):
        if token[0].startswith("{{"):
            return "{" + token[0][2:-2].strip().split(".")[-1] + "}"
        return token[0]

    yield ASKAMA.sub(parameter, action)


def segments(pattern):
    return ["{}" if PARAM.fullmatch(part) and part != "{lang}" else part for part in pattern.split("/")]


def inventory_route(action, method, routes):
    candidates = []
    actual = segments(action)
    for route in routes:
        pattern = route["route"]
        expected = segments(pattern)
        if len(expected) != len(actual):
            continue
        if all(a == b or b == "{}" and a not in {"", "{lang}"} for a, b in zip(actual, expected)):
            candidates.append(route)
    return max(candidates, key=lambda row: (
        [part != "{}" for part in segments(row["route"])], method in row["methods"],
    ), default=None)


def generate():
    routes = json.loads((LEDGER.parent / "inventory.json").read_text())["routes"]
    mutations = json.loads((LEDGER.parent / "mutation" / "forms.json").read_text())
    registrations = {(row["route"].replace("{locale}", "{lang}"), method)
                     for row in routes for method in row["methods"]}
    for row in mutations["routes"]:
        if (row["route"].replace("{locale}", "{lang}"), row["method"]) not in registrations:
            crate = row["sources"][0]["source"].split("/")[1]
            routes.append(dict(route=row["route"], methods=[row["method"]], crate=crate))
    forms = []
    for template in sorted(p for root in ("templates", "crates", "bin") for p in (PORTAL / root).rglob("*.html")):
        filename = template.relative_to(PORTAL).as_posix()
        if filename in ROUTE_ONLY_RENDERERS:
            continue
        parsed = Forms(template.read_text(encoding="utf-8")).forms
        overrides = ACTIONS.get(filename, [()] * len(parsed))
        if len(overrides) != len(parsed):
            raise ValueError(f"{filename} form mappings need updating")
        for form, actions in zip(parsed, overrides):
            destinations = {}
            for override, method, button in form["buttons"] or [(None, None, None)]:
                verb = (method or form["method"]).upper()
                for action in tuple(expand(override)) if override else actions or tuple(expand(form["action"])):
                    action = action.partition("?")[0]
                    if not action.startswith("/") or "{%" in action:
                        raise ValueError(f"{filename}:{form['line']} unresolved form action {action!r}")
                    buttons = destinations.setdefault((action, verb), [])
                    if button not in buttons:
                        buttons.append(button)
            for (action, method), buttons in sorted(destinations.items()):
                route = inventory_route(action, method, routes)
                if route and segments(route["route"]) == segments(action):
                    action = route["route"]
                # Inventory is provisional. Preserve forms on routes it omits.
                parts = template.relative_to(PORTAL).parts
                crate = route["crate"] if route else {
                    "templates/admin": "admin", "templates/content": "content",
                    "templates/profile": "characters",
                }.get("/".join(parts[:2]), parts[1] if parts[0] == "crates" else "web")
                entry = dict(template=filename, line=form["line"], action=action, method=method,
                             route_pattern=route["route"] if route else None, crate=crate,
                             fields=sorted(form["fields"]), buttons=buttons)
                if filename == CALCULATOR:
                    value = form["hidden"].get("calculator", "")
                    if not re.fullmatch(r"[a-z]+", value):
                        raise ValueError(f"{filename}:{form['line']} missing calculator operation")
                    entry["operation"] = dict(name="calculator", value=value)
                forms.append(entry)
    # The media editor's literal Rust markup is already extracted by the
    # mutation generator. Its actions use a query only for the return locale.
    for row in mutations["forms"]:
        filename, line = row["source"].rsplit(":", 1)
        if filename != "crates/content/src/media.rs":
            continue
        for action in expand(row["action"]):
            action = action.partition("?")[0]
            route = inventory_route(action, "POST", routes)
            if route is None or "POST" not in route["methods"]:
                raise ValueError(f"{row['source']} media form has no POST route")
            forms.append(dict(template=filename, line=int(line), action=route["route"], method="POST",
                              route_pattern=route["route"], crate=route["crate"],
                              fields=sorted(row["fields"]), buttons=[None]))
    locations = {(form["template"], form["line"]) for form in forms if form["method"] == "POST"}
    for row in mutations["forms"]:
        filename, line = row["source"].rsplit(":", 1)
        if (filename, int(line)) not in locations and filename not in ROUTE_ONLY_RENDERERS:
            raise ValueError(f"{row['source']} mutation form needs a ledger mapping or classification")
    return json.dumps(forms, indent=2, ensure_ascii=False) + "\n"


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    output = LEDGER / "forms.json"
    generated = generate()
    if args.check:
        if not output.is_file() or output.read_text(encoding="utf-8") != generated:
            parser.exit(1, "ledger/forms.json is stale; rerun ledger/forms.py\n")
    else:
        output.write_text(generated, encoding="utf-8")
