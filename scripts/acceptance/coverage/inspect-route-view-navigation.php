<?php

declare(strict_types=1);

use Illuminate\Contracts\Console\Kernel;
use Illuminate\Routing\Route;

$repoRoot = dirname(__DIR__, 3);
require $repoRoot.'/vendor/autoload.php';

$app = require $repoRoot.'/bootstrap/app.php';
$app->make(Kernel::class)->bootstrap();

function relativePath(string $file, string $repoRoot): string
{
    $normalizedFile = str_replace('\\', '/', $file);
    $normalizedRoot = rtrim(str_replace('\\', '/', $repoRoot), '/').'/';

    return str_starts_with($normalizedFile, $normalizedRoot)
        ? substr($normalizedFile, strlen($normalizedRoot))
        : $normalizedFile;
}

/**
 * @return array{views:list<array{name:string,file:string,line:int}>,sources:list<array{file:string,start_line:int,end_line:int,callable:string}>,has_redirect:bool,has_resource_response:bool,has_dynamic_view:bool}
 */
function inspectCallable(ReflectionFunctionAbstract $reflection, string $repoRoot, ArrayObject $seen): array
{
    $file = $reflection->getFileName();
    $key = ($file ?: '<internal>').':'.$reflection->getStartLine().':'.$reflection->getName();
    if (isset($seen[$key])) {
        return [
            'views' => [],
            'sources' => [],
            'has_redirect' => false,
            'has_resource_response' => false,
            'has_dynamic_view' => false,
        ];
    }
    $seen[$key] = true;

    if (is_string($file) === false || is_file($file) === false) {
        return [
            'views' => [],
            'sources' => [],
            'has_redirect' => false,
            'has_resource_response' => false,
            'has_dynamic_view' => false,
        ];
    }

    $lines = file($file, FILE_IGNORE_NEW_LINES);
    $start = max(1, $reflection->getStartLine());
    $end = max($start, $reflection->getEndLine());
    $sourceLines = array_slice($lines ?: [], $start - 1, $end - $start + 1);
    $source = implode("\n", $sourceLines);
    $relativeFile = relativePath($file, $repoRoot);
    $callable = $reflection instanceof ReflectionMethod
        ? $reflection->getDeclaringClass()->getName().'@'.$reflection->getName()
        : $reflection->getName();

    $views = [];
    $viewPatterns = [
        '/(?<![A-Za-z0-9_])view\s*\(\s*([\'\"])([^\'\"]+)\1/u',
        '/(?:View::make|response\(\)->view|->view)\s*\(\s*([\'\"])([^\'\"]+)\1/u',
    ];
    foreach ($viewPatterns as $pattern) {
        if (preg_match_all($pattern, $source, $matches, PREG_OFFSET_CAPTURE) < 1) {
            continue;
        }
        foreach ($matches[2] as [$name, $offset]) {
            $views[$name.'@'.$relativeFile.':'.($start + substr_count(substr($source, 0, $offset), "\n"))] = [
                'name' => $name,
                'file' => $relativeFile,
                'line' => $start + substr_count(substr($source, 0, $offset), "\n"),
            ];
        }
    }

    $hasRedirect = (bool) preg_match('/(?:\bredirect\s*\(|\bto_route\s*\(|Redirect::|->route\s*\()/u', $source);
    $hasResourceResponse = (bool) preg_match('/(?:response\(\)->(?:json|file|download|stream|streamDownload|noContent)|Storage::download|BinaryFileResponse|StreamedResponse|JsonResponse|Sitemap|robots\.txt)/u', $source);
    $hasDynamicView = (bool) preg_match('/(?<![A-Za-z0-9_])view\s*\(\s*(?![\'\"])/u', $source)
        || (bool) preg_match('/(?:View::make|response\(\)->view|->view)\s*\(\s*(?![\'\"])/u', $source);

    if ($reflection instanceof ReflectionMethod) {
        $class = $reflection->getDeclaringClass();
        if (preg_match_all('/(?:\$this->|self::|static::)([A-Za-z_][A-Za-z0-9_]*)\s*\(/u', $source, $calledMethods)) {
            foreach (array_unique($calledMethods[1]) as $methodName) {
                if ($methodName === $reflection->getName() || $class->hasMethod($methodName) === false) {
                    continue;
                }
                $nested = inspectCallable($class->getMethod($methodName), $repoRoot, $seen);
                foreach ($nested['views'] as $view) {
                    $views[$view['name'].'@'.$view['file'].':'.$view['line']] = $view;
                }
                $hasRedirect = $hasRedirect || $nested['has_redirect'];
                $hasResourceResponse = $hasResourceResponse || $nested['has_resource_response'];
                $hasDynamicView = $hasDynamicView || $nested['has_dynamic_view'];
            }
        }
    }

    return [
        'views' => array_values($views),
        'sources' => [[
            'file' => $relativeFile,
            'start_line' => $start,
            'end_line' => $end,
            'callable' => $callable,
        ]],
        'has_redirect' => $hasRedirect,
        'has_resource_response' => $hasResourceResponse,
        'has_dynamic_view' => $hasDynamicView,
    ];
}

function reflectRouteAction(Route $route): ?ReflectionFunctionAbstract
{
    $uses = $route->getAction('uses');
    if ($uses instanceof Closure) {
        return new ReflectionFunction($uses);
    }

    if (is_array($uses) && count($uses) === 2) {
        return new ReflectionMethod($uses[0], (string) $uses[1]);
    }

    if (is_string($uses) === false || $uses === '') {
        return null;
    }

    if (str_contains($uses, '@')) {
        [$class, $method] = explode('@', $uses, 2);
        return class_exists($class) && method_exists($class, $method)
            ? new ReflectionMethod($class, $method)
            : null;
    }

    return class_exists($uses) && method_exists($uses, '__invoke')
        ? new ReflectionMethod($uses, '__invoke')
        : null;
}

$records = [];
foreach ($app['router']->getRoutes() as $route) {
    if (($route instanceof Route) === false) {
        continue;
    }

    $name = trim((string) $route->getName());
    if ($name === '') {
        continue;
    }

    $reflection = reflectRouteAction($route);
    $seen = new ArrayObject;
    $inspection = $reflection
        ? inspectCallable($reflection, $repoRoot, $seen)
        : [
            'views' => [],
            'sources' => [],
            'has_redirect' => false,
            'has_resource_response' => false,
            'has_dynamic_view' => false,
        ];

    $defaults = property_exists($route, 'defaults') && is_array($route->defaults)
        ? $route->defaults
        : [];
    $safeDefaults = [];
    foreach (['view', 'destination', 'status'] as $key) {
        if (array_key_exists($key, $defaults) && (is_string($defaults[$key]) || is_int($defaults[$key]))) {
            $safeDefaults[$key] = $defaults[$key];
        }
    }

    if (isset($safeDefaults['view']) && is_string($safeDefaults['view'])) {
        $inspection['views'][] = [
            'name' => $safeDefaults['view'],
            'file' => 'routes/runtime-defaults',
            'line' => 1,
        ];
    }

    $records[] = array_merge([
        'name' => $name,
        'methods' => array_values(array_filter($route->methods(), static fn (string $method): bool => $method !== 'HEAD')),
        'uri' => $route->uri(),
        'action' => $route->getActionName(),
        'middleware' => array_values($route->gatherMiddleware()),
        'defaults' => $safeDefaults,
    ], $inspection);
}

usort($records, static fn (array $left, array $right): int => $left['name'] <=> $right['name']);

echo json_encode([
    'schema_version' => 1,
    'routes' => $records,
], JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES | JSON_THROW_ON_ERROR)."\n";
