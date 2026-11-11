<?php

namespace App\CharacterAuthority\Restore;

use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersWire;
use InvalidArgumentException;

/**
 * Wire of Platform contract `oteryn-v2-game-restore-reconciliation-v1`: the one Game restore
 * notice (Game OPERABILITY §3 ruling 10, amendment A3) and Platform's status answer, which carries
 * the acknowledgement once every operation the notice reopened has a new Game receipt.
 */
final class GameRestoreNoticeWire
{
    public const CONTRACT_VERSION = 1;

    public const OPERATION = 'PublishGameRestoreNoticeV1';

    public const PATH = '/internal/v1/game-auth/game-restore/notice';

    public const MAX_REQUEST_BYTES = 512;

    public const RECONCILING = 'RECONCILING';

    public const ACKNOWLEDGED = 'ACKNOWLEDGED';

    /** Generations are stored zero-padded to this width so they compare as text (§5.4). */
    public const GENERATION_WIDTH = 20;

    private const BACKUP_LABEL = '/^[A-Za-z0-9._:-]{1,128}$/D';

    private const MEMBERS = [
        'contract_version', 'operation', 'source_authority', 'restore_event_id', 'restored_generation',
        'new_recovery_generation', 'restore_point', 'backup_label',
    ];

    public static function decode(string $raw): GameRestoreNotice
    {
        $notice = NativeAccountCharactersWire::decodeExact($raw, self::MAX_REQUEST_BYTES);
        NativeAccountCharactersWire::exactKeys($notice, self::MEMBERS);
        if (($notice['contract_version'] ?? null) !== self::CONTRACT_VERSION || ($notice['operation'] ?? null) !== self::OPERATION) {
            throw new InvalidArgumentException('Game restore notice contract version or operation is invalid.');
        }

        $restored = NativeAccountCharactersWire::uint64($notice, 'restored_generation');
        $new = NativeAccountCharactersWire::uint64($notice, 'new_recovery_generation');
        if (NativeAccountCharactersWire::compareUint64($new, $restored) <= 0) {
            throw new InvalidArgumentException('Game restore notice must advance the recovery generation.');
        }

        return new GameRestoreNotice(
            sourceAuthority: NativeAccountCharactersWire::authority($notice),
            restoreEventId: NativeAccountCharactersWire::uuid7($notice, 'restore_event_id'),
            restoredGeneration: $restored,
            newRecoveryGeneration: $new,
            restorePoint: NativeAccountCharactersWire::uint64($notice, 'restore_point'),
            backupLabel: NativeAccountCharactersWire::text($notice, 'backup_label', self::BACKUP_LABEL),
        );
    }

    public static function encodeStatus(string $restoreEventId, int $reopened, int $outstanding): string
    {
        return json_encode([
            'contract_version' => self::CONTRACT_VERSION,
            'restore_event_id' => $restoreEventId,
            'status' => $outstanding === 0 ? self::ACKNOWLEDGED : self::RECONCILING,
            'reopened' => $reopened,
            'outstanding' => $outstanding,
        ], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES);
    }

    public static function padGeneration(string $generation): string
    {
        return str_pad($generation, self::GENERATION_WIDTH, '0', STR_PAD_LEFT);
    }
}
