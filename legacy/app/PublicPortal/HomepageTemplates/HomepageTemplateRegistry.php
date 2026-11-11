<?php

namespace App\PublicPortal\HomepageTemplates;

final class HomepageTemplateRegistry
{
    public const DEFAULT_KEY = 'production';

    /**
     * @var array<string, array{view: view-string, label: string, description: string}>
     */
    private const TEMPLATES = [
        'production' => [
            'view' => 'home',
            'label' => 'homepage_templates.templates.production.label',
            'description' => 'homepage_templates.templates.production.description',
        ],
        'classic' => [
            'view' => 'home-classic',
            'label' => 'homepage_templates.templates.classic.label',
            'description' => 'homepage_templates.templates.classic.description',
        ],
    ];

    /**
     * @return array<string, array{view: view-string, label: string, description: string}>
     */
    public function all(): array
    {
        return self::TEMPLATES;
    }

    /** @return list<string> */
    public function keys(): array
    {
        return array_keys(self::TEMPLATES);
    }

    public function has(string $key): bool
    {
        return array_key_exists($key, self::TEMPLATES);
    }

    public function resolve(?string $key): string
    {
        return $key !== null && $this->has($key)
            ? $key
            : self::DEFAULT_KEY;
    }

    /** @return view-string */
    public function view(string $key): string
    {
        return self::TEMPLATES[$this->resolve($key)]['view'];
    }
}
