<?php

namespace App\CanaryIntegration;

use Illuminate\Support\Facades\DB;
use RuntimeException;

final class CanaryCharacterTransferDatabasePrivilegeVerifier
{
    /** @var array<string, array<string, list<string>>> */
    private const APPROVED = [
        'accounts' => [
            'SELECT' => ['id'],
            'UPDATE' => [],
        ],
        'players' => [
            'SELECT' => CanaryCharacterTransfer::PLAYER_SELECT_COLUMNS,
            'UPDATE' => ['account_id'],
        ],
        'cluster_sessions' => [
            'SELECT' => ['player_id', 'account_id', 'status', 'expires_at'],
            'UPDATE' => [],
        ],
    ];

    /** @return list<string> */
    public function inspect(): array
    {
        $connection = DB::connection(CanaryCharacterTransfer::CONNECTION);
        $database = $connection->getDatabaseName();

        if ($database === '') {
            throw new RuntimeException('The Canary character-transfer database name is unavailable.');
        }

        $grants = [];
        foreach ($connection->select('SHOW GRANTS FOR CURRENT_USER') as $row) {
            $value = array_values((array) $row)[0] ?? null;
            if (! is_string($value) || $value === '') {
                throw new RuntimeException('The database returned an unreadable character-transfer grant row.');
            }
            $grants[] = $value;
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
            return ['No grants were returned for the current Canary character-transfer credential.'];
        }

        $violations = [];
        /** @var array<string, array<string, array<string, true>>> $found */
        $found = [];

        foreach ($grants as $index => $grant) {
            $number = $index + 1;
            $normalized = trim(rtrim($grant, ';'));

            if (preg_match('/\bWITH\s+GRANT\s+OPTION\b/i', $normalized) === 1) {
                $violations[] = "Grant #{$number} includes GRANT OPTION.";

                continue;
            }

            if (preg_match('/^GRANT\s+USAGE\s+ON\s+\*\.\*\s+TO\s+/i', $normalized) === 1) {
                continue;
            }

            if (preg_match('/^GRANT\s+(.+?)\s+ON\s+(.+?)\s+TO\s+/i', $normalized, $matches) !== 1) {
                $violations[] = "Grant #{$number} has an unsupported grant shape.";

                continue;
            }

            $target = $this->parseQualifiedTarget(trim($matches[2]));
            if ($target === null) {
                $violations[] = "Grant #{$number} has an unsupported privilege target.";

                continue;
            }

            [$grantDatabase, $table] = $target;
            if ($grantDatabase !== $database || ! array_key_exists($table, self::APPROVED)) {
                $violations[] = "Grant #{$number} targets data outside the approved Character Bazaar transfer surface.";

                continue;
            }

            $privileges = trim($matches[1]);
            $select = $this->extractColumnPrivilege($privileges, 'SELECT');
            $update = $this->extractColumnPrivilege($privileges, 'UPDATE');
            $remaining = preg_replace('/\bSELECT\s*\([^)]*\)/i', '', $privileges, 1);
            $remaining = is_string($remaining) ? preg_replace('/\bUPDATE\s*\([^)]*\)/i', '', $remaining, 1) : null;
            $remaining = is_string($remaining) ? trim(str_replace(',', '', $remaining)) : null;

            if ($remaining === null || $remaining !== '' || ($select === null && $update === null)) {
                $violations[] = "Grant #{$number} includes an unsupported or non-column-level privilege.";

                continue;
            }

            foreach (['SELECT' => $select, 'UPDATE' => $update] as $privilege => $columns) {
                if ($columns === null) {
                    continue;
                }

                foreach ($columns as $column) {
                    if (! in_array($column, self::APPROVED[$table][$privilege], true)) {
                        $violations[] = "Grant #{$number} grants {$privilege} on unapproved {$table}.{$column}.";

                        continue 3;
                    }
                    $found[$table][$privilege][$column] = true;
                }
            }
        }

        foreach (self::APPROVED as $table => $privileges) {
            foreach ($privileges as $privilege => $columns) {
                foreach ($columns as $column) {
                    if (! isset($found[$table][$privilege][$column])) {
                        $violations[] = "Missing approved {$privilege} privilege for {$table}.{$column}.";
                    }
                }
            }
        }

        return array_values(array_unique($violations));
    }

    /** @return list<string>|null */
    private function extractColumnPrivilege(string $privileges, string $privilege): ?array
    {
        if (preg_match('/\b'.preg_quote($privilege, '/').'\s*\(([^)]*)\)/i', $privileges, $matches) !== 1) {
            return null;
        }

        $columns = [];
        foreach (explode(',', $matches[1]) as $column) {
            $column = trim($column);
            if (preg_match('/^`(?:``|[^`])+`$|^[A-Za-z0-9_$]+$/', $column) !== 1) {
                return null;
            }
            $columns[] = $this->unquoteIdentifier($column);
        }

        return array_values(array_unique($columns));
    }

    /** @return array{0: string, 1: string}|null */
    private function parseQualifiedTarget(string $target): ?array
    {
        if (preg_match('/^(?<database>`(?:``|[^`])+`|[A-Za-z0-9_$]+)\.(?<table>`(?:``|[^`])+`|[A-Za-z0-9_$]+)$/', $target, $matches) !== 1) {
            return null;
        }

        return [$this->unquoteIdentifier($matches['database']), $this->unquoteIdentifier($matches['table'])];
    }

    private function unquoteIdentifier(string $identifier): string
    {
        return str_starts_with($identifier, '`') && str_ends_with($identifier, '`')
            ? str_replace('``', '`', substr($identifier, 1, -1))
            : $identifier;
    }
}
