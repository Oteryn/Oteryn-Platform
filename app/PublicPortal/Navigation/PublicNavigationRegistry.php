<?php

namespace App\PublicPortal\Navigation;

use Illuminate\Support\Facades\Route;

/**
 * @phpstan-type NavigationItem array{label: string, url: string, active: string, priority: int}
 * @phpstan-type FooterGroup array{label: string, priority: int, items: list<NavigationItem>}
 */
final class PublicNavigationRegistry
{
    /** @return list<NavigationItem> */
    public function header(): array
    {
        $items = [];

        foreach ($this->definitions() as $definition) {
            $header = $definition['header'] ?? [];
            if (! is_array($header)) {
                continue;
            }

            foreach ($header as $item) {
                $normalized = $this->normalizeItem($item);
                if ($normalized !== null) {
                    $items[] = $normalized;
                }
            }
        }

        usort($items, self::compareItems(...));

        return $items;
    }

    /** @return list<FooterGroup> */
    public function footer(): array
    {
        /** @var array<string, FooterGroup> $groups */
        $groups = [];

        foreach ($this->definitions() as $definition) {
            $footer = $definition['footer'] ?? [];
            if (! is_array($footer)) {
                continue;
            }

            foreach ($footer as $group) {
                if (! is_array($group)) {
                    continue;
                }

                $key = $group['key'] ?? null;
                $label = $group['label'] ?? null;
                $priority = $group['priority'] ?? 100;
                $groupItems = $group['items'] ?? [];

                if (! is_string($key) || $key === '' || ! is_string($label) || $label === '' || ! is_int($priority) || ! is_array($groupItems)) {
                    continue;
                }

                $groups[$key] ??= [
                    'label' => (string) __($label),
                    'priority' => $priority,
                    'items' => [],
                ];

                foreach ($groupItems as $item) {
                    $normalized = $this->normalizeItem($item);
                    if ($normalized !== null) {
                        $groups[$key]['items'][] = $normalized;
                    }
                }
            }
        }

        $sortedGroups = [];
        foreach ($groups as $group) {
            usort($group['items'], self::compareItems(...));
            if ($group['items'] !== []) {
                $sortedGroups[] = $group;
            }
        }

        usort($sortedGroups, self::compareGroups(...));

        return $sortedGroups;
    }

    /** @return list<array<string, mixed>> */
    private function definitions(): array
    {
        $files = glob(resource_path('navigation/public/*.php'));
        if ($files === false) {
            return [];
        }

        sort($files, SORT_STRING);
        $definitions = [];

        foreach ($files as $file) {
            $definition = require $file;
            if (is_array($definition)) {
                /** @var array<string, mixed> $definition */
                $definitions[] = $definition;
            }
        }

        return $definitions;
    }

    /** @return NavigationItem|null */
    private function normalizeItem(mixed $item): ?array
    {
        if (! is_array($item)) {
            return null;
        }

        $label = $item['label'] ?? null;
        $route = $item['route'] ?? null;
        $active = $item['active'] ?? $route;
        $priority = $item['priority'] ?? 100;
        $fragment = $item['fragment'] ?? null;

        if (! is_string($label) || $label === '' || ! is_string($route) || $route === '' || ! is_string($active) || $active === '' || ! is_int($priority) || ! Route::has($route)) {
            return null;
        }

        if ($fragment !== null && (! is_string($fragment) || $fragment === '')) {
            return null;
        }

        $urlRoute = $route;
        $parameters = [];
        if ($route === 'home' && Route::has('localized.home')) {
            $urlRoute = 'localized.home';
            $parameters['locale'] = app()->getLocale();
            $active = 'home|localized.home';
        }

        $url = route($urlRoute, $parameters);
        if (is_string($fragment)) {
            $url .= '#'.rawurlencode($fragment);
        }

        return [
            'label' => (string) __($label),
            'url' => $url,
            'active' => $active,
            'priority' => $priority,
        ];
    }

    /**
     * @param  NavigationItem  $left
     * @param  NavigationItem  $right
     */
    private static function compareItems(array $left, array $right): int
    {
        return $left['priority'] <=> $right['priority'];
    }

    /**
     * @param  FooterGroup  $left
     * @param  FooterGroup  $right
     */
    private static function compareGroups(array $left, array $right): int
    {
        return $left['priority'] <=> $right['priority'];
    }
}
