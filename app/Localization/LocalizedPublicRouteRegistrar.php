<?php

namespace App\Localization;

use Closure;
use Illuminate\Routing\Route as LaravelRoute;
use Illuminate\Routing\Router;
use LogicException;

final readonly class LocalizedPublicRouteRegistrar
{
    public function __construct(private Router $router, private PublicLocale $locales) {}

    public function register(): void
    {
        $this->router->getRoutes()->refreshNameLookups();

        $home = $this->requiredRoute('home');
        $homeUses = $this->routeUses($home, 'home');
        $home->middleware('public.locale.negotiate');

        /** @var array<string, string> $definitions */
        $definitions = [
            'today.index' => '/today',
            'news.index' => '/news',
            'news.show' => '/news/{slug}',
            'pages.show' => '/pages/{slug}',
            'game.highscores.index' => '/highscores',
            'game.characters.search' => '/characters',
            'game.characters.show' => '/characters/{name}',
            'game.guilds.index' => '/guilds',
            'game.guilds.show' => '/guilds/{name}',
            'game.deaths.index' => '/deaths',
            'game.online.index' => '/online',
            'game.servers.index' => '/servers',
            'events.index' => '/events',
            'events.show' => '/events/{slug}',
            'downloads.index' => '/download/{platform?}',
            'editorial.getting-started' => '/getting-started',
            'editorial.server-information' => '/server-information',
            'support.index' => '/support',
            'support.report-a-bug' => '/support/report-a-bug',
            'editorial.rules' => '/rules',
            'legal.terms' => '/legal/terms',
            'legal.privacy' => '/legal/privacy',
            'legal.cookies' => '/legal/cookies',
            'game-catalog.index' => '/wiki/catalog',
            'game-catalog.items.index' => '/wiki/items',
            'game-catalog.items.show' => '/wiki/items/{slug}',
            'game-catalog.creatures.index' => '/wiki/creatures',
            'game-catalog.creatures.show' => '/wiki/creatures/{slug}',
            'wiki.index' => '/wiki',
            'wiki.search' => '/wiki/search',
            'wiki.category' => '/wiki/category/{slug}',
            'wiki.article' => '/wiki/{slug}',
        ];

        if (config('marketplace.enabled')) {
            $definitions['marketplace.index'] = '/bazaar';
            $definitions['marketplace.show'] = '/bazaar/{auction}';
        }

        /** @var array<string, array{uses: Closure|array<array-key, mixed>|string, defaults: array<string, mixed>, wheres: array<string, string>, middleware: array<int, string>}> $sourceRoutes */
        $sourceRoutes = [];

        foreach ($definitions as $name => $uri) {
            $source = $this->requiredRoute($name);
            $uses = $this->routeUses($source, $name);

            $sourceRoutes[$name] = [
                'uses' => $uses,
                'defaults' => $source->defaults,
                'wheres' => $source->wheres,
                'middleware' => $source->middleware(),
            ];

            $action = $source->getAction();
            if (! is_array($action)) {
                throw new LogicException(sprintf('Public route [%s] has no supported route action.', $name));
            }

            $action['as'] = 'legacy.'.$name;
            $source->setAction($action);
            $source->defaults('locale', $this->locales->default());
            $source->middleware('public.locale');
        }

        $this->router->getRoutes()->refreshNameLookups();

        $this->router
            ->get('/{locale}', $this->normalizeInvokable($homeUses))
            ->where('locale', 'en|pl')
            ->middleware(['web', 'public.locale'])
            ->name('localized.home');

        foreach ($sourceRoutes as $name => $definition) {
            $route = $this->router
                ->get('/{locale}'.$definitions[$name], $this->normalizeInvokable($definition['uses']))
                ->where('locale', 'en|pl')
                ->defaults('locale', $this->locales->default())
                ->middleware([...$definition['middleware'], 'public.locale'])
                ->name($name);

            foreach ($definition['defaults'] as $key => $value) {
                if ($key !== 'locale') {
                    $route->defaults($key, $value);
                }
            }

            if ($definition['wheres'] !== []) {
                $route->where($definition['wheres']);
            }
        }
    }

    private function requiredRoute(string $name): LaravelRoute
    {
        $route = $this->router->getRoutes()->getByName($name);
        if (! $route instanceof LaravelRoute) {
            throw new LogicException(sprintf('Required public route [%s] is not registered.', $name));
        }

        return $route;
    }

    /** @return Closure|array<array-key, mixed>|string */
    private function routeUses(LaravelRoute $route, string $name): Closure|array|string
    {
        $action = $route->getAction();
        $uses = is_array($action) ? ($action['uses'] ?? null) : null;
        if (! is_string($uses) && ! is_array($uses) && ! $uses instanceof Closure) {
            throw new LogicException(sprintf('Public route [%s] has no supported route action.', $name));
        }

        return $uses;
    }

    /**
     * @param  Closure|array<array-key, mixed>|string  $uses
     * @return Closure|array<array-key, mixed>|string
     */
    private function normalizeInvokable(Closure|array|string $uses): Closure|array|string
    {
        return is_string($uses) && str_ends_with($uses, '@__invoke')
            ? substr($uses, 0, -strlen('@__invoke'))
            : $uses;
    }
}
