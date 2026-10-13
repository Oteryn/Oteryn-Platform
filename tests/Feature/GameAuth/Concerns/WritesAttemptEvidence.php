<?php

namespace Tests\Feature\GameAuth\Concerns;

use App\GameAuth\NativeEvidence\NativeEvidenceContract;

/**
 * Fixture writer for the attempt evidence directory (PostgreSQL P6a design note D3 and D4): one jsonl
 * file per run, `prev` = sha256 of the previous line's bytes without the newline, `seq` from 1.
 */
trait WritesAttemptEvidence
{
    private const FIXTURE_RUN = '11111111-2222-4333-8444-555555555555';

    private const FIXTURE_KEY_ID = 'kid-1';

    private const FIXTURE_PURPOSE = 'fresh_admission';

    /**
     * @param  list<array<string, mixed>>  $records  Each has `type`; explicit v/seq/prev override the defaults.
     * @return list<string> line hashes, in order
     */
    private function writeRun(string $directory, string $runId, array $records): array
    {
        $previous = null;
        $hashes = [];
        $out = '';
        foreach ($records as $index => $record) {
            if (($record['type'] ?? null) === 'anchored') {
                $record += ['for_seq' => $index, 'for_line_sha256' => $previous, 'anchor_comment_id' => 1001];
            }
            $line = $record + ['v' => 1, 'run_id' => $runId, 'seq' => $index + 1, 'prev' => $previous, 'at' => '2026-10-13T10:00:00Z'];
            $json = (string) json_encode($line, JSON_UNESCAPED_SLASHES);
            $previous = hash('sha256', $json);
            $hashes[] = $previous;
            $out .= $json."\n";
        }
        file_put_contents($directory.'/'.$runId.'.jsonl', $out);

        return $hashes;
    }

    /** @return array<string, string> */
    private function fixtureScope(): array
    {
        return [
            'issuer' => NativeEvidenceContract::FRESH_ISSUER,
            'profile' => NativeEvidenceContract::FRESH_PROFILE,
            'key_purpose' => self::FIXTURE_PURPOSE,
        ];
    }

    /** @return array<string, mixed> */
    private function fixtureDatabase(string $lsn = '0/1000000', string $xmax = '700'): array
    {
        return [
            'identity' => [
                'system_identifier' => '7000000000000000001',
                'timeline_id' => 1,
                'postmaster_start_time' => '2026-10-01T00:00:00.000000Z',
                'database_oid' => 16384,
                'relfilenodes' => [
                    'native_game_signing_trust_profiles' => 20001,
                    'native_game_signing_trust_key_versions' => 20002,
                    'native_game_evidence_witness_stores' => 20003,
                ],
            ],
            'progress' => ['wal_flush_lsn' => $lsn, 'xmax' => $xmax],
            'in_flight' => ['sessions' => 0, 'prepared' => 0],
        ];
    }

    /** @param array<string, mixed>|null $database */
    private function fixtureObservation(?array $database, ?string $storeId): array
    {
        return [
            'result' => 'observed',
            'fence_required' => true,
            'store' => ['marker' => $storeId, 'row' => $storeId, 'binding' => $storeId === null ? 'none' : 'match'],
            'scope' => $this->fixtureScope(),
            'floor' => ['value' => null, 'temp_entries' => 0],
            'profile' => ['present' => false],
            'key' => ['requested' => self::FIXTURE_KEY_ID, 'configured_key_id' => self::FIXTURE_KEY_ID, 'in_history' => false, 'trusted' => false],
            'database' => $database,
        ];
    }

    /**
     * The records of one clean run that ends with an anchored attempt intent.
     *
     * @param  array{initStore?: string, initDatabase?: array<string, mixed>, intentDatabase?: array<string, mixed>, intent?: array<string, mixed>, anchored?: array<string, mixed>, skipAnchored?: bool, after?: list<array<string, mixed>>}  $options
     * @return list<array<string, mixed>>
     */
    private function standardRecords(string $storeId, array $options = []): array
    {
        $intent = array_replace([
            'type' => 'attempt-intent',
            'attempt' => 1,
            'store_id' => $storeId,
            'scope' => $this->fixtureScope(),
            'key_id' => self::FIXTURE_KEY_ID,
            'kid_record_count' => 1,
            'observation' => $this->fixtureObservation($options['intentDatabase'] ?? $this->fixtureDatabase('0/2000000', '710'), $storeId),
        ], $options['intent'] ?? []);

        $records = [
            ['type' => 'run-open', 'store_id' => null, 'pre_init' => $this->fixtureObservation(null, null)],
            ['type' => 'init-intent'],
            ['type' => 'anchored'],
            [
                'type' => 'init',
                'store_id' => $options['initStore'] ?? $storeId,
                'result' => 'created',
                'observation' => $this->fixtureObservation($options['initDatabase'] ?? $this->fixtureDatabase(), $storeId),
            ],
            $intent,
        ];
        if (($options['skipAnchored'] ?? false) !== true) {
            $records[] = ['type' => 'anchored'] + ($options['anchored'] ?? []);
        }

        return array_merge($records, $options['after'] ?? []);
    }
}
