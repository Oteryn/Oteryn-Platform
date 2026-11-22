"""Inventory HTML forms in Askama templates using only the standard library."""

import json
import re
from html.parser import HTMLParser
from pathlib import Path

LEDGER = Path(__file__).resolve().parent
PORTAL = LEDGER.parent.parent

# Shared templates and implicit actions derive their destinations from Rust
# renderers. Keep the concrete alternatives for each form here.
ACTIONS = {
    "bin/oteryn-portal/templates/code_issuance.html": [
        ("/{lang}/admin/store/codes",),
    ],
    "crates/admin/src/console/templates/page.html": [
        ("/console/dashboard", "/console/worlds", "/console/nodes", "/console/database"),
    ] * 2,
    "crates/admin/src/console/templates/read.html": [
        ("/admin/console/players",), ("/admin/console/players",), ("/admin/console/audit",),
    ],
    "crates/payments/templates/coin/checkout.html": [
        ("/native-account/coin-checkout/{order_id}",),
    ] * 2,
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

ASKAMA = re.compile(r"{{.*?}}|{%.*?%}|{#.*?#}", re.S)
PARAM = re.compile(r"{([^{}]+)}")
CONDITIONAL = re.compile(r"{%\s*if\b.*?%}(.*?)(?:{%\s*else\s*%}(.*?))?{%\s*endif\s*%}")
CALCULATOR = "templates/wiki/calculators/page.html"


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
    conditional = CONDITIONAL.search(action)
    if conditional:
        for value in (conditional[1], conditional[2] or ""):
            yield from expand(action[:conditional.start()] + value + action[conditional.end():])
        return
    yield re.sub(r"{{\s*(.*?)\s*}}", lambda m: "{" + m[1].split(".")[-1] + "}", action)


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
    forms = []
    for template in sorted(p for p in PORTAL.rglob("*.html") if "templates" in p.relative_to(PORTAL).parts):
        filename = template.relative_to(PORTAL).as_posix()
        parsed = Forms(template.read_text(encoding="utf-8")).forms
        overrides = ACTIONS.get(filename, [()] * len(parsed))
        if len(overrides) != len(parsed):
            raise ValueError(f"{filename} form mappings need updating")
        for form, actions in zip(parsed, overrides):
            destinations = {}
            for override, method, button in form["buttons"] or [(None, None, None)]:
                verb = (method or form["method"]).upper()
                for action in tuple(expand(override)) if override else actions or tuple(expand(form["action"])):
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
    (LEDGER / "forms.json").write_text(json.dumps(forms, indent=2, ensure_ascii=False) + "\n",
                                       encoding="utf-8")


if __name__ == "__main__":
    generate()
