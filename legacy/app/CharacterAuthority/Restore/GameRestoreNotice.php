<?php

namespace App\CharacterAuthority\Restore;

/** A decoded `PublishGameRestoreNoticeV1` request; generations are canonical uint64 strings. */
final readonly class GameRestoreNotice
{
    public function __construct(
        public string $sourceAuthority,
        public string $restoreEventId,
        public string $restoredGeneration,
        public string $newRecoveryGeneration,
        public string $restorePoint,
        public string $backupLabel,
    ) {}
}
