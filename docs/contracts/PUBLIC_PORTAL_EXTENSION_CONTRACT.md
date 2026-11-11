# Public Portal Extension Contract

## Purpose

This contract keeps future public website slices parallelizable. Feature agents add module-local routes, navigation contributions and implementations without editing the central route bootstrap, shared public layout or permission registry for the keys already reserved here.

## Route registration

`legacy/routes/web.php` loads every PHP file under `legacy/routes/modules/*.php` in deterministic filename order.

Rules:

- each feature owns one module-local file, for example `legacy/routes/modules/downloads.php`;
- route names must be globally unique and module-prefixed where practical;
- a module route file may register only routes implemented by that module;
- do not add placeholder routes or links;
- do not edit `legacy/routes/web.php` for a normal module addition;
- privileged routes still require `auth`, `mfa.confirmed` and one exact `admin.permission:<key>` middleware.

The PublicPortal foundation owns `legacy/routes/modules/public-portal.php` and the production `/` route.

## Public navigation registration

`App\PublicPortal\Navigation\PublicNavigationRegistry` loads repository-controlled definitions from `legacy/resources/navigation/public/*.php` in deterministic filename order.

A contribution may contain `header` items and/or `footer` groups. Items use:

```php
[
    'label' => 'Downloads',
    'route' => 'downloads.index',
    'active' => 'downloads.*',
    'priority' => 60,
]
```

Optional `fragment` appends an encoded fragment to the named route URL.

Safety behavior:

- items are rendered only when the named route currently exists;
- definitions contain route names, never arbitrary URLs or HTML;
- navigation labels are escaped by Blade;
- external links require a separate trusted-configuration and validation decision;
- feature agents add a module-owned file such as `legacy/resources/navigation/public/downloads.php` rather than editing the shared header, footer or `core.php`.

## Reserved exact permissions

The following keys are registered in `AdminPermission` and persisted in `admin_permissions`:

```text
portal.access
portal.announcements.manage
portal.settings.manage
downloads.manage
events.manage
events.publish
support.content.manage
wiki.access
wiki.articles.manage
wiki.categories.manage
wiki.publish
```

Reservation does not grant authority. The reservation migration intentionally adds no `admin_role_permissions` rows. A later feature may use only the exact key appropriate to its operation and must separately justify any explicit role-bundle change.

Wildcard permissions and implicit future authority remain prohibited.

## Shared path ownership

The PublicPortal foundation owns:

- `legacy/routes/web.php` module loader;
- `legacy/routes/modules/public-portal.php`;
- `legacy/app/PublicPortal/**` shared homepage and navigation composition;
- `legacy/app/Http/Controllers/PublicPortal/PublicHomeController.php`;
- `legacy/resources/views/game/layout.blade.php` and `resources/views/game/partials/public-*.blade.php`;
- `legacy/resources/navigation/public/core.php`;
- `legacy/resources/views/home.blade.php`;
- `legacy/public/css/public-shell.css` and `legacy/public/css/home-production.css`;
- the reserved permission constants and reservation migration.

Future feature ownership:

| Feature | Module-local route | Navigation contribution | Implementation namespace |
|---|---|---|---|
| Downloads | `legacy/routes/modules/downloads.php` | `legacy/resources/navigation/public/downloads.php` | `legacy/app/Downloads/**` |
| Events | `legacy/routes/modules/events.php` | `legacy/resources/navigation/public/events.php` | `legacy/app/Events/**` |
| Support | `legacy/routes/modules/support.php` | `legacy/resources/navigation/public/support.php` | `legacy/app/Support/**` |
| Wiki | `legacy/routes/modules/wiki.php` | `legacy/resources/navigation/public/wiki.php` | `legacy/app/Wiki/**` |
| Additional public game data | `routes/modules/public-game-data-<feature>.php` | `resources/navigation/public/public-game-data-<feature>.php` | approved `legacy/app/PublicGameData/**` feature paths |

Those agents must not modify the foundation-owned shared files unless a separately coordinated contract change is required.
