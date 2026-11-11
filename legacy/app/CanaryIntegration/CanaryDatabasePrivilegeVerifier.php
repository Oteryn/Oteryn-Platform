<?php

namespace App\CanaryIntegration;

use Illuminate\Support\Facades\DB;
use RuntimeException;

final class CanaryDatabasePrivilegeVerifier
{
    /**
     * Keep this list synchronized with the tables actually read through the
     * dedicated `canary` connection and with database/provisioning/.
     *
     * @var list<string>
     */
    public const REQUIRED_TABLES = [
        'players',
        'guilds',
        'guild_membership',
        'guild_ranks',
        'houses',
        'player_deaths',
        'channels',
        'cluster_sessions',
    ];

    /**
     * Inspect grants for the credential behind the configured Canary connection.
     *
     * This performs metadata reads only. It never attempts a write to prove that
     * the credential is read-only.
     *
     * @return list<string> Sanitized policy violations; an empty list means pass.
     */
    public function inspect(): array
    {
        $connection = DB::connection('canary');
        $database = $connection->getDatabaseName();

        if ($database === '') {
            throw new RuntimeException('The Canary database name is unavailable.');
        }

        $rows = $connection->select('SHOW GRANTS FOR CURRENT_USER');
        $grants = [];

        foreach ($rows as $row) {
            $values = array_values((array) $row);
            $grant = $values[0] ?? null;

            if (! is_string($grant) || $grant === '') {
                throw new RuntimeException('The database returned an unreadable grant row.');
            }

            $grants[] = $grant;
        }

        return $this->verify($database, $grants);
    }

    /**
     * @param  list<string>  $grants
     * @return list<string>
     */
    public function verify(string $database, array $grants): array
    {
        if ($grants === []) {
            return ['No grants were returned for the current Canary database credential.'];
        }

        $violations = [];
        $selectedTables = [];

        foreach ($grants as $index => $grant) {
            $grantNumber = $index + 1;
            $normalized = trim(rtrim($grant, ';'));

            if (preg_match('/\bWITH\s+GRANT\s+OPTION\b/i', $normalized) === 1) {
                $violations[] = "Grant #{$grantNumber} includes GRANT OPTION.";

                continue;
            }

            // MySQL/MariaDB commonly emit an account-level USAGE row. The tail
            // may contain authentication metadata, so never echo the raw row.
            if (preg_match('/^GRANT\s+USAGE\s+ON\s+\*\.\*\s+TO\s+/i', $normalized) === 1) {
                continue;
            }

            if (preg_match('/^GRANT\s+(.+?)\s+ON\s+(.+?)\s+TO\s+/i', $normalized, $matches) !== 1) {
                $violations[] = "Grant #{$grantNumber} has an unsupported grant shape.";

                continue;
            }

            if (strcasecmp(trim($matches[1]), 'SELECT') !== 0) {
                $violations[] = "Grant #{$grantNumber} includes a privilege other than direct SELECT.";

                continue;
            }

            $target = $this->parseQualifiedTarget(trim($matches[2]));

            if ($target === null) {
                $violations[] = "Grant #{$grantNumber} has an unsupported privilege target.";

                continue;
            }

            [$grantDatabase, $table] = $target;

            if ($grantDatabase === '*' || $table === '*') {
                $violations[] = "Grant #{$grantNumber} is global or schema-wide; only direct table SELECT grants are allowed.";

                continue;
            }

            if ($grantDatabase !== $database) {
                $violations[] = "Grant #{$grantNumber} targets a database outside the configured Canary database.";

                continue;
            }

            if (! in_array($table, self::REQUIRED_TABLES, true)) {
                $violations[] = "Grant #{$grantNumber} targets a table outside the approved Canary read allowlist.";

                continue;
            }

            $selectedTables[$table] = true;
        }

        foreach (self::REQUIRED_TABLES as $table) {
            if (! isset($selectedTables[$table])) {
                $violations[] = "Missing direct SELECT grant for required Canary table: {$table}.";
            }
        }

        return array_values(array_unique($violations));
    }

    /**
     * @return array{0: string, 1: string}|null
     */
    private function parseQualifiedTarget(string $target): ?array
    {
        if (preg_match(
            '/^(?<database>`(?:``|[^`])+`|[A-Za-z0-9_$]+|\*)\.(?<table>`(?:``|[^`])+`|[A-Za-z0-9_$]+|\*)$/',
            $target,
            $matches,
        ) !== 1) {
            return null;
        }

        return [
            $this->unquoteIdentifier($matches['database']),
            $this->unquoteIdentifier($matches['table']),
        ];
    }

    private function unquoteIdentifier(string $identifier): string
    {
        if (str_starts_with($identifier, '`') && str_ends_with($identifier, '`')) {
            return str_replace('``', '`', substr($identifier, 1, -1));
        }

        return $identifier;
    }
}
