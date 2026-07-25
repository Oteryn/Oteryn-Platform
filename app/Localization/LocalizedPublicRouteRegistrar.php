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

        /** @var array<string, string> $definitions */
        $definitions = [
            'home' => '',
            'news.index' => '/news',
            'news.show' => '/news/{slug}',
            'pages.show' => '/pages/{slug}',
            'game.highscores.index' => '/highscores',
            'game.characters.search' => '/characters',
            'game.characters.show' => '/characters/{name}',
            'game.guilds.index' => '/guilds',
            'game.guilds.show' => '/guilds/{name}',
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
        ];

        /** @var array<string, array{uses: Closure|array<array-key, mixed>|string, defaults: array<string, mixed>, wheres: array<string, string>}> $sourceRoutes */
        $sourceRoutes = [];

        foreach ($definitions as $name => $uri) {
            $source = $this->router->getRoutes()->getByName($name);
            if (! $source instanceof LaravelRoute) {
                throw new LogicException(sprintf('Required public route [%s] is not registered.', $name));
            }

            $action = $source->getAction();
            if (! is_array($action)) {
                throw new LogicException(sprintf('Public route [%s] has no supported route action.', $name));
            }

            $uses = $action['uses'] ?? null;
            if (! is_string($uses) && ! is_array($uses) && ! $uses instanceof Closure) {
                throw new LogicException(sprintf('Public route [%s] has no supported route action.', $name));
            }

            $sourceRoutes[$name] = [
                'uses' => $uses,
                'defaults' => $source->defaults,
                'wheres' => $source->wheres,
            ];

            $action['as'] = 'legacy.'.$name;
            $source->setAction($action);
            $source->defaults('locale', $this->locales->default());
            $source->middleware($name === 'home' ? 'public.locale.negotiate' : 'public.locale');
        }

        $this->router->getRoutes()->refreshNameLookups();

        foreach ($sourceRoutes as $name => $definition) {
            $uses = $definition['uses'];
            if (is_string($uses) && str_ends_with($uses, '@__invoke')) {
                $uses = substr($uses, 0, -strlen('@__invoke'));
            }

            $route = $this->router
                ->get('/{locale}'.($definitions[$name] === '' ? '' : $definitions[$name]), $uses)
                ->where('locale', 'en|pl')
                ->defaults('locale', $this->locales->default())
                ->middleware(['web', 'public.locale'])
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
}
