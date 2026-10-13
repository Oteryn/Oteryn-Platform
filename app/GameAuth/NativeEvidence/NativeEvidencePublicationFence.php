<?php

namespace App\GameAuth\NativeEvidence;

use JsonException;
use LogicException;

/**
 * First-publication fence (design note PLATFORM_POSTGRES_TRUST_SAFEGUARDS_DESIGN, D8). The caller names
 * only a run id and an intent hash; every value the producer trusts is read from the attempt-intent line
 * in the read-only evidence directory (jsonl, one run per file, hash chain = sha256 of the line bytes
 * without the newline, `prev`, `seq`; design note D3 and D4).
 */
final class NativeEvidencePublicationFence
{
    private const PERMANENT_REFUSALS = ['store-changed', 'evidence-lost', 'init-lost'];

    private const BLOCKING_CLASSIFICATIONS = ['failed-after-witness', 'restore-or-loss', 'unknown'];

    private const MAX_FILE_BYTES = 8_388_608;

    private ?string $issuer = null;

    private ?string $profile = null;

    private ?string $keyPurpose = null;

    private ?string $keyId = null;

    /** @var array{store_id: string, attempt: int, identity: array<string, mixed>, wal_flush_lsn: string, xmax: string}|null */
    private ?array $verified = null;

    public function __construct(
        public readonly string $runId,
        public readonly string $intentSha256,
    ) {
        if (preg_match('/\A[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\z/', $runId) !== 1
            || preg_match('/\A[0-9a-f]{64}\z/', $intentSha256) !== 1) {
            throw new NativeEvidenceUnavailable('The first publication fence is malformed.');
        }
    }

    /** Parses base64url(JSON {"run_id", "intent_sha256"}). */
    public static function fromEncoded(string $encoded): self
    {
        $json = preg_match('/\A[A-Za-z0-9_-]{1,2048}\z/', $encoded) === 1
            ? base64_decode(strtr($encoded, '-_', '+/'), true)
            : false;
        if (! is_string($json)) {
            throw new NativeEvidenceUnavailable('The first publication fence is malformed.');
        }
        try {
            $data = json_decode($json, true, 4, JSON_THROW_ON_ERROR);
        } catch (JsonException) {
            throw new NativeEvidenceUnavailable('The first publication fence is malformed.');
        }
        if (! is_array($data) || array_keys($data) !== ['run_id', 'intent_sha256']
            || ! is_string($data['run_id']) || ! is_string($data['intent_sha256'])) {
            throw new NativeEvidenceUnavailable('The first publication fence is malformed.');
        }

        return new self($data['run_id'], $data['intent_sha256']);
    }

    /** `required` or `off` (default); any other value fails closed. */
    public static function required(): bool
    {
        $value = config('game-auth.native_evidence.first_publication_fence');
        if ($value === null || $value === 'off') {
            return false;
        }
        if ($value === 'required') {
            return true;
        }

        throw new LogicException('The first publication fence setting must be required or off.');
    }

    /** Binds the call's scope and key id; the verified intent must carry exactly these. */
    public function bind(string $issuer, string $profile, string $keyPurpose, string $keyId): self
    {
        $this->issuer = $issuer;
        $this->profile = $profile;
        $this->keyPurpose = $keyPurpose;
        $this->keyId = $keyId;
        $this->verified = null;

        return $this;
    }

    /** @return array{run_id: string, attempt: int, intent_sha256: string}|null Null until verify() passed. */
    public function result(): ?array
    {
        if ($this->verified === null) {
            return null;
        }

        return ['run_id' => $this->runId, 'attempt' => $this->verified['attempt'], 'intent_sha256' => $this->intentSha256];
    }

    /**
     * Reads every evidence file and refuses unless the named intent is the one open, anchored intent for
     * this store, scope and key id and every directory-wide rule of D8 step 3 holds.
     */
    public function verify(string $storeId): void
    {
        $this->verified = null;
        if ($this->keyId === null) {
            throw new LogicException('The first publication fence is not bound to a call.');
        }
        $runs = $this->readDirectory();

        $openIntents = [];
        $stableIdentity = null;
        $observations = [];
        foreach ($runs as $runId => $lines) {
            $pendingInit = false;
            $openAttempts = [];
            foreach ($lines as $line) {
                $type = $line['data']['type'];
                $data = $line['data'];
                foreach (['observation', 'pre_init', 'observation_1', 'observation_2'] as $key) {
                    if (isset($data[$key]) && is_array($data[$key])) {
                        $observations[] = $data[$key];
                    }
                }
                if ($type === 'init-intent') {
                    $pendingInit = true;
                } elseif ($type === 'init' || $type === 'begin-refused') {
                    $pendingInit = false;
                }
                if ($type === 'init') {
                    if (($data['store_id'] ?? null) !== $storeId) {
                        throw new NativeEvidenceUnavailable('The evidence directory holds an init for another witness store.');
                    }
                    $identity = $data['observation']['database']['identity'] ?? null;
                    if ($stableIdentity === null && is_array($identity)) {
                        $stableIdentity = NativeEvidenceDatabaseIdentity::stable($identity);
                    }
                }
                if ($type === 'begin-refused' && in_array($data['reason'] ?? null, self::PERMANENT_REFUSALS, true)) {
                    throw new NativeEvidenceUnavailable('The evidence directory holds a permanent refusal.');
                }
                if ($type === 'attempt-intent') {
                    $openAttempts[$line['sha']] = $data['attempt'] ?? null;
                    $openIntents[] = $line['sha'];
                }
                if ($type === 'attempt-observation') {
                    if (in_array($data['classification'] ?? null, self::BLOCKING_CLASSIFICATIONS, true)) {
                        throw new NativeEvidenceUnavailable('The evidence directory holds a blocking attempt classification.');
                    }
                    foreach ($openAttempts as $sha => $attempt) {
                        if ($attempt === ($data['attempt'] ?? null)) {
                            unset($openAttempts[$sha]);
                            $openIntents = array_values(array_diff($openIntents, [$sha]));
                        }
                    }
                }
            }
            if ($pendingInit) {
                throw new NativeEvidenceUnavailable('The evidence directory holds an unfinished init intent.');
            }
        }

        if ($openIntents !== [$this->intentSha256]) {
            throw new NativeEvidenceUnavailable('The named intent is not the only open attempt intent.');
        }
        if (! isset($runs[$this->runId])) {
            throw new NativeEvidenceUnavailable('The named evidence run does not exist.');
        }
        $lines = $runs[$this->runId];
        $count = count($lines);
        $anchored = $lines[$count - 1] ?? null;
        $intent = $lines[$count - 2] ?? null;
        if ($count < 2 || $anchored['data']['type'] !== 'anchored' || $intent['data']['type'] !== 'attempt-intent'
            || ! hash_equals($intent['sha'], $this->intentSha256)
            || ($anchored['data']['for_seq'] ?? null) !== $intent['data']['seq']
            || ! is_string($anchored['data']['for_line_sha256'] ?? null)
            || ! hash_equals($anchored['data']['for_line_sha256'], $intent['sha'])
            || ! $this->hasCommentId($anchored['data']['anchor_comment_id'] ?? null)) {
            throw new NativeEvidenceUnavailable('The named intent is not anchored.');
        }

        $intentData = $intent['data'];
        $scope = $intentData['scope'] ?? null;
        $observation = $intentData['observation'] ?? null;
        $attempt = $intentData['attempt'] ?? null;
        if (($intentData['store_id'] ?? null) !== $storeId
            || ($intentData['key_id'] ?? null) !== $this->keyId
            || ! is_array($scope)
            || ($scope['issuer'] ?? null) !== $this->issuer
            || ($scope['profile'] ?? null) !== $this->profile
            || ($scope['key_purpose'] ?? null) !== $this->keyPurpose
            || ! is_int($attempt) || $attempt < 1
            || ! is_array($observation)) {
            throw new NativeEvidenceUnavailable('The named intent does not match this store, scope and key id.');
        }
        $database = $observation['database'] ?? null;
        $floorObservation = $observation['floor'] ?? null;
        if (! is_array($floorObservation) || ! array_key_exists('value', $floorObservation) || $floorObservation['value'] !== null
            || ($observation['profile']['present'] ?? null) !== false
            || ! is_array($database)
            || ! is_array($database['identity'] ?? null)
            || ! NativeEvidenceDatabaseIdentity::validLsn($database['progress']['wal_flush_lsn'] ?? null)
            || ! is_string($database['progress']['xmax'] ?? null)
            || preg_match('/\A[0-9]{1,20}\z/', $database['progress']['xmax']) !== 1) {
            throw new NativeEvidenceUnavailable('The named intent observation is not a clean database baseline.');
        }

        // Database continuity (D4): same stable identity as the first init, progress not below any observation.
        if ($stableIdentity === null
            || NativeEvidenceDatabaseIdentity::stable($database['identity']) !== $stableIdentity) {
            throw new NativeEvidenceUnavailable('The named intent does not continue the initialised witness store.');
        }
        $baselineLsn = $database['progress']['wal_flush_lsn'];
        foreach ($observations as $other) {
            $otherLsn = $other['database']['progress']['wal_flush_lsn'] ?? null;
            if ($otherLsn !== null && (! NativeEvidenceDatabaseIdentity::validLsn($otherLsn)
                || NativeEvidenceDatabaseIdentity::compareLsn($baselineLsn, $otherLsn) < 0)) {
                throw new NativeEvidenceUnavailable('The named intent is behind an earlier recorded database position.');
            }
        }

        $this->verified = [
            'store_id' => $storeId,
            'attempt' => $attempt,
            'identity' => NativeEvidenceDatabaseIdentity::sorted($database['identity']),
            'wal_flush_lsn' => $baselineLsn,
            'xmax' => $database['progress']['xmax'],
        ];
    }

    /** Inside the database transaction, before the floor advances. Locks tables, then compares. */
    public function assertDatabaseBound(NativeEvidenceDatabaseIdentity $database): void
    {
        $verified = $this->verified;
        if ($verified === null) {
            throw new NativeEvidenceUnavailable('The first publication fence was not verified.');
        }
        if (! $database->supported()) {
            throw new NativeEvidenceUnavailable('The first publication fence requires a PostgreSQL database identity.');
        }
        $database->lockTrackedTables();
        if (NativeEvidenceDatabaseIdentity::sorted($database->identity()) !== $verified['identity']) {
            throw new NativeEvidenceUnavailable('The database identity differs from the fenced intent.');
        }
        if (! $database->flushLsnAtLeast($verified['wal_flush_lsn']) || ! $database->xmaxAtLeast($verified['xmax'])) {
            throw new NativeEvidenceUnavailable('The database progress is below the fenced intent baseline.');
        }
    }

    /**
     * @return array<string, list<array{data: array<string, mixed>, sha: string}>> keyed by run id
     */
    private function readDirectory(): array
    {
        $configured = config('game-auth.native_evidence.attempt_evidence_directory');
        $directory = is_string($configured) && $configured !== '' ? realpath($configured) : false;
        if (! is_string($directory) || ! is_dir($directory) || ! is_readable($directory)) {
            throw new NativeEvidenceUnavailable('The attempt evidence directory is unavailable.');
        }
        $entries = scandir($directory);
        if ($entries === false) {
            throw new NativeEvidenceUnavailable('The attempt evidence directory is unavailable.');
        }

        $runs = [];
        foreach ($entries as $entry) {
            if (! str_ends_with($entry, '.jsonl')) {
                continue;
            }
            $runId = substr($entry, 0, -6);
            $path = $directory.DIRECTORY_SEPARATOR.$entry;
            if (preg_match('/\A[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\z/', $runId) !== 1
                || is_link($path) || ! is_file($path)) {
                throw new NativeEvidenceUnavailable('The attempt evidence directory holds an unexpected run file.');
            }
            $runs[$runId] = $this->readRun($path, $runId);
        }

        return $runs;
    }

    /** @return list<array{data: array<string, mixed>, sha: string}> */
    private function readRun(string $path, string $runId): array
    {
        $size = @filesize($path);
        $raw = is_int($size) && $size > 0 && $size <= self::MAX_FILE_BYTES ? @file_get_contents($path) : false;
        if (! is_string($raw) || ! str_ends_with($raw, "\n")) {
            throw new NativeEvidenceUnavailable('An attempt evidence run file is unreadable or incomplete.');
        }

        $lines = [];
        $previousSha = null;
        $previousSeq = null;
        foreach (explode("\n", substr($raw, 0, -1)) as $text) {
            try {
                $data = json_decode($text, true, 32, JSON_THROW_ON_ERROR);
            } catch (JsonException) {
                throw new NativeEvidenceUnavailable('An attempt evidence line is malformed.');
            }
            $seq = is_array($data) ? ($data['seq'] ?? null) : null;
            if (! is_array($data) || ($data['v'] ?? null) !== 1 || ($data['run_id'] ?? null) !== $runId
                || ! is_int($seq) || ! is_string($data['type'] ?? null) || ! is_string($data['at'] ?? null)
                || ! array_key_exists('prev', $data) || $data['prev'] !== $previousSha
                || ($previousSeq === null ? ($seq < 0 || $seq > 1) : $seq !== $previousSeq + 1)) {
                throw new NativeEvidenceUnavailable('An attempt evidence hash chain is broken.');
            }
            $sha = hash('sha256', $text);
            $lines[] = ['data' => $data, 'sha' => $sha];
            $previousSha = $sha;
            $previousSeq = $seq;
        }

        return $lines;
    }

    private function hasCommentId(mixed $value): bool
    {
        return (is_int($value) && $value > 0) || (is_string($value) && $value !== '');
    }
}
