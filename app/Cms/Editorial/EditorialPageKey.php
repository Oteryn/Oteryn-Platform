<?php

namespace App\Cms\Editorial;

enum EditorialPageKey: string
{
    case GettingStarted = 'getting-started';
    case ServerInformation = 'server-information';
    case Support = 'support';
    case ReportABug = 'report-a-bug';
    case Rules = 'rules';
    case Terms = 'terms';
    case Privacy = 'privacy';
    case Cookies = 'cookies';

    public function managedPageSlug(): string
    {
        return match ($this) {
            self::GettingStarted => 'getting-started',
            self::ServerInformation => 'server-information',
            self::Support => 'support',
            self::ReportABug => 'report-a-bug',
            self::Rules => 'rules',
            self::Terms => 'terms-of-service',
            self::Privacy => 'privacy-policy',
            self::Cookies => 'cookie-policy',
        };
    }

    public function publicRouteName(): string
    {
        return match ($this) {
            self::GettingStarted => 'editorial.getting-started',
            self::ServerInformation => 'editorial.server-information',
            self::Support => 'support.index',
            self::ReportABug => 'support.report-a-bug',
            self::Rules => 'editorial.rules',
            self::Terms => 'legal.terms',
            self::Privacy => 'legal.privacy',
            self::Cookies => 'legal.cookies',
        };
    }

    public function label(): string
    {
        return match ($this) {
            self::GettingStarted => 'Beginner\'s Guide',
            self::ServerInformation => 'Server Information',
            self::Support => 'Support',
            self::ReportABug => 'Report a Bug',
            self::Rules => 'Rules',
            self::Terms => 'Terms of Service',
            self::Privacy => 'Privacy Policy',
            self::Cookies => 'Cookie Policy',
        };
    }

    /**
     * @return list<string>
     */
    public function expectedTopics(): array
    {
        return match ($this) {
            self::GettingStarted => ['Installation', 'First login'],
            self::ServerInformation => ['Server information', 'Rates', 'PvP rules'],
            self::Support => ['Account security', 'MFA guide', 'FAQ', 'Known issues', 'Contact and support'],
            self::ReportABug => ['Known-issue check', 'Report-a-bug instructions', 'Personal-data warning', 'Contact and support'],
            self::Rules => ['Game rules', 'Naming rules', 'Prohibited software', 'PvP rules'],
            self::Terms => ['Terms of Service'],
            self::Privacy => ['Privacy Policy'],
            self::Cookies => ['Cookie Policy'],
        };
    }

    public function isLegal(): bool
    {
        return in_array($this, [self::Terms, self::Privacy, self::Cookies], true);
    }

    public function isSupportGuidance(): bool
    {
        return in_array($this, [self::Support, self::ReportABug], true);
    }

    /**
     * @return list<string>
     */
    public static function managedPageSlugs(): array
    {
        return array_map(
            static fn (self $key): string => $key->managedPageSlug(),
            self::cases(),
        );
    }

    public static function fromManagedPageSlug(string $slug): ?self
    {
        foreach (self::cases() as $key) {
            if ($key->managedPageSlug() === $slug) {
                return $key;
            }
        }

        return null;
    }
}
