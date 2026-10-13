<?php

namespace App\GameAuth\NativeEvidence;

use Illuminate\Support\Facades\DB;

/**
 * Identity, progress and in-flight reads of the PostgreSQL database behind the trust tables (design
 * note PLATFORM_POSTGRES_TRUST_SAFEGUARDS_DESIGN, D2 and D8). Every function used works for a
 * non-superuser role. Other drivers have no identity: every read returns null.
 */
final class NativeEvidenceDatabaseIdentity
{
    /** @var list<string> */
    public const TRACKED_TABLES = [
        'native_game_signing_trust_profiles',
        'native_game_signing_trust_key_versions',
        'native_game_evidence_witness_stores',
    ];

    public function supported(): bool
    {
        return DB::connection()->getDriverName() === 'pgsql';
    }

    /**
     * @return array{identity: array<mixed, mixed>, progress: array{wal_flush_lsn: string, xmax: string}, in_flight: array{sessions: int, prepared: int}}|null
     */
    public function read(): ?array
    {
        if (! $this->supported()) {
            return null;
        }

        return [
            'identity' => $this->identity(),
            'progress' => $this->progress(),
            'in_flight' => $this->inFlight(),
        ];
    }

    /** @return array<mixed, mixed> */
    public function identity(): array
    {
        $system = self::scalar(DB::selectOne('select system_identifier::text as system_identifier from pg_control_system()'), 'system_identifier');
        $checkpoint = self::scalar(DB::selectOne('select timeline_id from pg_control_checkpoint()'), 'timeline_id');
        $postmaster = self::scalar(DB::selectOne(
            "select to_char(pg_postmaster_start_time() at time zone 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"') as started",
        ), 'started');
        $database = self::scalar(DB::selectOne('select oid::text as oid from pg_database where datname = current_database()'), 'oid');

        $filenodes = [];
        foreach (self::TRACKED_TABLES as $table) {
            $node = self::scalar(DB::selectOne('select pg_relation_filenode(to_regclass(?))::text as node', [$table]), 'node');
            $filenodes[$table] = $node !== '' ? (int) $node : null;
        }

        return [
            'system_identifier' => $system,
            'timeline_id' => (int) $checkpoint,
            'postmaster_start_time' => $postmaster,
            'database_oid' => (int) $database,
            'relfilenodes' => $filenodes,
        ];
    }

    /** @return array{wal_flush_lsn: string, xmax: string} */
    public function progress(): array
    {
        $row = DB::selectOne(
            'select pg_current_wal_flush_lsn()::text as lsn, pg_snapshot_xmax(pg_current_snapshot())::text as xmax',
        );

        return [
            'wal_flush_lsn' => self::scalar($row, 'lsn'),
            'xmax' => self::scalar($row, 'xmax'),
        ];
    }

    /** @return array{sessions: int, prepared: int} */
    public function inFlight(): array
    {
        $sessions = DB::selectOne(
            'select count(*) as n from pg_stat_activity where datname = current_database() '
            .'and pid <> pg_backend_pid() and usename = current_user and xact_start is not null',
        );
        $prepared = DB::selectOne('select count(*) as n from pg_prepared_xacts');

        return [
            'sessions' => (int) self::scalar($sessions, 'n'),
            'prepared' => (int) self::scalar($prepared, 'n'),
        ];
    }

    /** Inside the fenced transaction: conflicts with the locks TRUNCATE, DROP and table rewrites need. */
    public function lockTrackedTables(): void
    {
        foreach (self::TRACKED_TABLES as $table) {
            DB::statement('LOCK TABLE '.$table.' IN ROW EXCLUSIVE MODE');
        }
    }

    public function flushLsnAtLeast(string $baseline): bool
    {
        if (! self::validLsn($baseline)) {
            return false;
        }
        $row = DB::selectOne('select (pg_current_wal_flush_lsn() >= ?::pg_lsn) as ok', [$baseline]);

        return filter_var(self::scalar($row, 'ok'), FILTER_VALIDATE_BOOL);
    }

    public function xmaxAtLeast(string $baseline): bool
    {
        if (preg_match('/\A[0-9]{1,20}\z/', $baseline) !== 1) {
            return false;
        }
        $row = DB::selectOne('select (pg_snapshot_xmax(pg_current_snapshot()) >= ?::xid8) as ok', [$baseline]);

        return filter_var(self::scalar($row, 'ok'), FILTER_VALIDATE_BOOL);
    }

    /** One column of a `selectOne` row as text; an absent row, column or non-scalar value reads as ''. */
    private static function scalar(mixed $row, string $column): string
    {
        $values = is_object($row) ? get_object_vars($row) : [];
        $value = $values[$column] ?? null;

        return is_scalar($value) ? (string) $value : '';
    }

    /** @phpstan-assert-if-true string $value */
    public static function validLsn(mixed $value): bool
    {
        return is_string($value) && preg_match('/\A[0-9A-Fa-f]{1,8}\/[0-9A-Fa-f]{1,8}\z/', $value) === 1;
    }

    /** Compares two valid LSN texts: negative, zero or positive. */
    public static function compareLsn(string $left, string $right): int
    {
        [$leftHigh, $leftLow] = array_map('hexdec', explode('/', $left));
        [$rightHigh, $rightLow] = array_map('hexdec', explode('/', $right));

        return [$leftHigh, $leftLow] <=> [$rightHigh, $rightLow];
    }

    /**
     * Identity without postmaster_start_time, key-sorted, for the stable-identity comparison across runs.
     *
     * @param  array<mixed, mixed>  $identity
     * @return array<mixed, mixed>
     */
    public static function stable(array $identity): array
    {
        unset($identity['postmaster_start_time']);

        return self::sorted($identity);
    }

    /**
     * @param  array<mixed, mixed>  $value
     * @return array<mixed, mixed>
     */
    public static function sorted(array $value): array
    {
        ksort($value);
        foreach ($value as $key => $item) {
            if (is_array($item)) {
                $value[$key] = self::sorted($item);
            }
        }

        return $value;
    }
}
