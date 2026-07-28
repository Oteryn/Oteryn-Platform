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

    'tickets' => [
        'subject_max_length' => 160,
        'message_max_length' => 8000,
        'open_limit_per_identity' => 10,
        'retention_days_after_close' => 730,
    ],
    'reports' => [
        'target_max_length' => 160,
        'evidence_max_length' => 4000,
        'pending_limit_per_identity' => 5,
        'retention_days_after_close' => 730,
    ],
    'enforcement' => [
        'reason_max_length' => 4000,
        'appeal_max_length' => 4000,
        'retention_days_after_expiry' => 1095,
    ],
    'attachments' => [
        'enabled' => false,
        'reason' => 'No reviewed support-attachment upload model is currently delivered.',
    ],
];
