<?php

namespace App\GameAuth\NativeRuntimeStatus;

use InvalidArgumentException;
use JsonException;

/**
 * One `ReportRuntimeStatusV1` body (Game `oteryn-game-native-runtime-status-v1` §4), parsed exactly as
 * the Game encoder writes it: 22 members, no unknown, duplicate, missing, null or nested member, and each
 * value inside the Game grammar. Epochs, generations and revisions are uint64 and stay canonical decimal
 * strings. The report never carries an endpoint; the World Registry owns the route (§2).
 */
final readonly class NativeRuntimeStatusReport
{
    public const MAX_REQUEST_BYTES = 2048;

    private const UINT64_MAX = '18446744073709551615';

    private const UUID7 = '/^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/D';

    private const UUID = '/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/D';

    private const REVISION = '/^[A-Za-z0-9._:-]{1,64}$/D';

    /** Wire members other than contract_version, operation and observed_at, with their grammar. */
    private const CONTENT = [
        'source_authority' => '/^[A-Za-z0-9._:\/-]{1,128}$/D',
        'world_id' => self::UUID7,
        'channel_id' => self::UUID7,
        'node_id' => self::UUID,
        'assignment_epoch' => 'uint64',
        'scope_ownership_generation' => 'uint64',
        'source_revision' => 'uint64',
        'decision_identity' => '/^[A-Za-z0-9._:-]{1,128}$/D',
        'ready' => 'bool',
        'published_at' => 'time',
        'protocol_major' => 'one',
        'transport_profile' => 'one',
        'route_revision' => self::REVISION,
        'runtime_observation_revision' => self::REVISION,
        'ruleset_revision' => self::REVISION,
        'content_revision' => self::REVISION,
        'map_revision' => self::REVISION,
        'world_policy_revision' => self::REVISION,
        'offer_revision' => self::REVISION,
    ];

    /**
     * @param  array<string, string|int|bool>  $content  every member except contract_version, operation and observed_at
     */
    private function __construct(
        public array $content,
        public string $worldId,
        public string $channelId,
        public string $assignmentEpoch,
        public string $scopeOwnershipGeneration,
        public string $sourceRevision,
        public int $observedAt,
    ) {}

    public static function fromWire(string $raw): self
    {
        $decoded = self::decode($raw, count(self::CONTENT) + 3, 'ReportRuntimeStatusV1');

        $content = [];
        foreach (self::CONTENT as $name => $grammar) {
            $content[$name] = self::member($decoded, $name, $grammar);
        }
        $publishedAt = self::member($decoded, 'published_at', 'time');
        $observedAt = self::member($decoded, 'observed_at', 'time');
        if (! is_int($publishedAt) || ! is_int($observedAt) || $observedAt < $publishedAt) {
            throw new InvalidArgumentException('Runtime status report is observed before it was published.');
        }
        $text = static fn (string $name, string $grammar): string => is_string($value = self::member($decoded, $name, $grammar))
            ? $value
            : throw new InvalidArgumentException('Runtime status report member is invalid.');

        return new self(
            $content,
            $text('world_id', self::UUID7),
            $text('channel_id', self::UUID7),
            $text('assignment_epoch', 'uint64'),
            $text('scope_ownership_generation', 'uint64'),
            $text('source_revision', 'uint64'),
            $observedAt,
        );
    }

    /**
     * The exact flat Game object with $members members (including contract_version and operation):
     * no unknown, duplicate (even an escaped spelling), missing, null or nested member.
     *
     * @return array<mixed>
     */
    public static function decode(string $raw, int $members, string $operation): array
    {
        if ($raw === '' || strlen($raw) > self::MAX_REQUEST_BYTES) {
            throw new InvalidArgumentException('Native runtime report size is invalid.');
        }
        // A duplicate (even an escaped spelling of a name) adds a member name that decoding would drop.
        preg_match_all('/"((?:[^"\\\\]|\\\\.)*)"\s*:/s', $raw, $names);
        if (count($names[1]) !== $members) {
            throw new InvalidArgumentException('Native runtime report member count is invalid.');
        }

        try {
            $decoded = json_decode($raw, true, 2, JSON_THROW_ON_ERROR);
        } catch (JsonException $exception) {
            throw new InvalidArgumentException('Native runtime report JSON is invalid.', 0, $exception);
        }
        if (! is_array($decoded)
            || count($decoded) !== $members
            || ($decoded['contract_version'] ?? null) !== 1
            || ($decoded['operation'] ?? null) !== $operation) {
            throw new InvalidArgumentException('Native runtime report envelope is invalid.');
        }

        return $decoded;
    }

    /** Numeric order of two canonical decimal strings. */
    public static function compare(string $left, string $right): int
    {
        return (strlen($left) <=> strlen($right)) ?: (strcmp($left, $right) <=> 0);
    }

    /** Identity of the publication: every member except observed_at (§7: equal key, different content). */
    public function contentDigest(): string
    {
        return hash('sha256', json_encode($this->content, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES));
    }

    /**
     * One member in the Game grammar: a regex, or `bool`, `one`, `uint64` or `time` (returned as int).
     *
     * @param  array<mixed>  $decoded
     */
    public static function member(array $decoded, string $name, string $grammar): string|int|bool
    {
        $value = $decoded[$name] ?? null;
        $valid = match ($grammar) {
            'bool' => is_bool($value),
            'one' => $value === 1,
            'uint64' => is_string($value)
                && preg_match('/^[1-9][0-9]{0,19}$/D', $value) === 1
                && self::compare($value, self::UINT64_MAX) <= 0,
            'time' => is_string($value)
                && preg_match('/^(0|[1-9][0-9]{0,18})$/D', $value) === 1
                && self::compare($value, (string) PHP_INT_MAX) <= 0,
            default => is_string($value) && preg_match($grammar, $value) === 1,
        };
        if (! $valid || ! (is_string($value) || is_int($value) || is_bool($value))) {
            throw new InvalidArgumentException('Runtime status report member is invalid.');
        }

        return $grammar === 'time' ? (int) $value : $value;
    }
}
