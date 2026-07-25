<?php

/** @var Closure(mixed): list<string> $hostList */
$hostList = static function (mixed $value): array {
    if (! is_string($value) || trim($value) === '') {
        return [];
    }

    return array_values(array_filter(array_map(
        static fn (string $host): string => trim($host),
        explode(',', $value),
    )));
};

return [
    'discord_url' => env('OTERYN_SUPPORT_DISCORD_URL'),
    'discord_hosts' => $hostList(env('OTERYN_SUPPORT_DISCORD_HOSTS', 'discord.gg,discord.com')),
    'contact_email' => env('OTERYN_SUPPORT_CONTACT_EMAIL'),
    'support_url' => env('OTERYN_SUPPORT_URL'),
    'allowed_hosts' => $hostList(env('OTERYN_SUPPORT_ALLOWED_HOSTS')),
];
