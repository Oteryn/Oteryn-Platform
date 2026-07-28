<?php

namespace App\CanaryIntegration;

use App\Marketplace\Contracts\CanaryCharacterTransferGateway;
use App\Marketplace\Data\CharacterOwnershipState;
use App\Marketplace\Data\CharacterSnapshot;
use App\Marketplace\Data\CharacterTransferResult;
use App\Marketplace\Exceptions\MarketplaceException;
use Illuminate\Database\ConnectionInterface;
use Illuminate\Database\QueryException;
use Illuminate\Support\Facades\DB;
use Throwable;

final class CanaryCharacterTransfer implements CanaryCharacterTransferGateway
{
    public const CONNECTION = 'canary_character_transfer';

    private const MAX_TRANSACTION_ATTEMPTS = 3;

    /** @var list<string> */
    public const PLAYER_SELECT_COLUMNS = [
        'id',
        'name',
        'account_id',
        'deletion',
        'level',
        'vocation',
        'experience',
        'sex',
        'maglevel',
        'skill_fist',
        'skill_club',
        'skill_sword',
        'skill_axe',
        'skill_dist',
        'skill_shielding',
        'skill_fishing',
        'looktype',
        'lookaddons',
        'lookhead',
        'lookbody',
        'looklegs',
        'lookfeet',
        'town_id',
        'lastlogin',
        'lastlogout',
    ];

    public function snapshotOwnedCharacter(int $sourceAccountId, int $playerId): CharacterSnapshot
    {
        try {
            $connection = DB::connection(self::CONNECTION);
            $row = $connection->table('players')
                ->select(self::PLAYER_SELECT_COLUMNS)
                ->where('id', $playerId)
                ->first();

            if ($row === null) {
                throw new MarketplaceException('character_missing', 'The selected character does not exist.');
            }

            $values = (array) $row;
            $accountId = $this->positiveInt($values, 'account_id');
            $deletion = $this->intValue($values, 'deletion');

            if ($accountId !== $sourceAccountId) {
                throw new MarketplaceException('ownership_conflict', 'The selected character is not owned by your bound game account.');
            }

            if ($deletion !== 0) {
                throw new MarketplaceException('character_deleted', 'The selected character is not active.');
            }

            if ($this->hasClusterSession($connection, $playerId)) {
                throw new MarketplaceException('character_online_or_session_active', 'Log the character out completely before listing it.');
            }

            $name = $values['name'] ?? null;
            if (! is_string($name) || trim($name) === '') {
                throw new MarketplaceException('dependency_unavailable', 'The character snapshot is unavailable.');
            }

            $publicData = [];
            foreach (self::PLAYER_SELECT_COLUMNS as $column) {
                if (in_array($column, ['account_id', 'deletion'], true)) {
                    continue;
                }

                $value = $values[$column] ?? null;
                if (! is_int($value) && ! is_string($value) && $value !== null) {
                    throw new MarketplaceException('dependency_unavailable', 'The character snapshot is unavailable.');
                }

                $publicData[$column] = $value;
            }

            return new CharacterSnapshot(
                playerId: $this->positiveInt($values, 'id'),
                accountId: $accountId,
                name: $name,
                deletion: $deletion,
                publicData: $publicData,
            );
        } catch (MarketplaceException $exception) {
            throw $exception;
        } catch (Throwable) {
            throw new MarketplaceException('dependency_unavailable', 'The character service is temporarily unavailable.');
        }
    }

    public function transfer(
        int $playerId,
        int $expectedSourceAccountId,
        int $targetAccountId,
        bool $enforceTargetCharacterLimit,
    ): CharacterTransferResult {
        for ($attempt = 1; $attempt <= self::MAX_TRANSACTION_ATTEMPTS; $attempt++) {
            try {
                return $this->attemptTransfer(
                    $playerId,
                    $expectedSourceAccountId,
                    $targetAccountId,
                    $enforceTargetCharacterLimit,
                );
            } catch (MarketplaceException $exception) {
                throw $exception;
            } catch (QueryException $exception) {
                if ($this->isTransientConcurrencyFailure($exception) && $attempt < self::MAX_TRANSACTION_ATTEMPTS) {
                    continue;
                }

                throw new MarketplaceException('dependency_unavailable', 'The character transfer service is temporarily unavailable.');
            } catch (Throwable) {
                throw new MarketplaceException('dependency_unavailable', 'The character transfer service is temporarily unavailable.');
            }
        }

        throw new MarketplaceException('dependency_unavailable', 'The character transfer service is temporarily unavailable.');
    }

    public function ownershipState(int $playerId): CharacterOwnershipState
    {
        try {
            $connection = DB::connection(self::CONNECTION);
            $row = $connection->table('players')
                ->select(['id', 'account_id', 'deletion'])
                ->where('id', $playerId)
                ->first();

            if ($row === null) {
                throw new MarketplaceException('character_missing', 'The marketplace character no longer exists.');
            }

            $values = (array) $row;

            return new CharacterOwnershipState(
                playerId: $this->positiveInt($values, 'id'),
                accountId: $this->positiveInt($values, 'account_id'),
                deletion: $this->intValue($values, 'deletion'),
                hasClusterSession: $this->hasClusterSession($connection, $playerId),
            );
        } catch (MarketplaceException $exception) {
            throw $exception;
        } catch (Throwable) {
            throw new MarketplaceException('dependency_unavailable', 'The character ownership state is temporarily unavailable.');
        }
    }

    private function attemptTransfer(
        int $playerId,
        int $expectedSourceAccountId,
        int $targetAccountId,
        bool $enforceTargetCharacterLimit,
    ): CharacterTransferResult {
        $connection = DB::connection(self::CONNECTION);

        return $connection->transaction(function () use (
            $connection,
            $playerId,
            $expectedSourceAccountId,
            $targetAccountId,
            $enforceTargetCharacterLimit,
        ): CharacterTransferResult {
            $accountIds = array_values(array_unique([$expectedSourceAccountId, $targetAccountId]));
            sort($accountIds, SORT_NUMERIC);

            $accountRows = $connection->table('accounts')
                ->select('id')
                ->whereIn('id', $accountIds)
                ->orderBy('id')
                ->lockForUpdate()
                ->get();
            $lockedAccountIds = [];

            foreach ($accountRows as $accountRow) {
                $lockedAccountIds[] = $this->positiveInt((array) $accountRow, 'id');
            }

            if (! in_array($expectedSourceAccountId, $lockedAccountIds, true)) {
                throw new MarketplaceException('source_account_missing', 'The source game account no longer exists.');
            }

            if (! in_array($targetAccountId, $lockedAccountIds, true)) {
                throw new MarketplaceException('target_account_missing', 'The target game account no longer exists.');
            }

            $player = $connection->table('players')
                ->select(['id', 'account_id', 'deletion'])
                ->where('id', $playerId)
                ->lockForUpdate()
                ->first();

            if ($player === null) {
                throw new MarketplaceException('character_missing', 'The marketplace character no longer exists.');
            }

            $values = (array) $player;
            $currentAccountId = $this->positiveInt($values, 'account_id');

            if ($currentAccountId === $targetAccountId) {
                return new CharacterTransferResult(
                    CharacterTransferResult::ALREADY_TRANSFERRED,
                    $playerId,
                    $targetAccountId,
                );
            }

            if ($currentAccountId !== $expectedSourceAccountId) {
                throw new MarketplaceException('ownership_conflict', 'The character ownership changed outside the marketplace operation.');
            }

            if ($this->intValue($values, 'deletion') !== 0) {
                throw new MarketplaceException('character_deleted', 'The marketplace character is not active.');
            }

            if ($this->hasClusterSession($connection, $playerId, true)) {
                throw new MarketplaceException('character_online_or_session_active', 'The character still has a game session.');
            }

            if ($enforceTargetCharacterLimit) {
                $limit = (int) config('marketplace.character_limit', 10);
                if ($limit < 1 || $limit > 100) {
                    throw new MarketplaceException('invalid_marketplace_configuration', 'The marketplace character limit configuration is invalid.');
                }

                $targetCount = $connection->table('players')
                    ->where('account_id', $targetAccountId)
                    ->where('deletion', 0)
                    ->where('id', '<>', $playerId)
                    ->count('id');

                if ($targetCount >= $limit) {
                    throw new MarketplaceException('target_character_limit', 'The target account has reached its active character limit.');
                }
            }

            $updated = $connection->table('players')
                ->where('id', $playerId)
                ->where('account_id', $expectedSourceAccountId)
                ->update(['account_id' => $targetAccountId]);

            if ($updated !== 1) {
                throw new MarketplaceException('ownership_conflict', 'The character ownership changed during transfer.');
            }

            return new CharacterTransferResult(
                CharacterTransferResult::TRANSFERRED,
                $playerId,
                $targetAccountId,
            );
        }, 1);
    }

    private function hasClusterSession(ConnectionInterface $connection, int $playerId, bool $lock = false): bool
    {
        $query = $connection->table('cluster_sessions')
            ->select(['player_id'])
            ->where('player_id', $playerId);

        if ($lock) {
            $query->lockForUpdate();
        }

        return $query->exists();
    }

    /** @param array<string, mixed> $values */
    private function positiveInt(array $values, string $key): int
    {
        $value = $values[$key] ?? null;
        if ((! is_int($value) && ! is_string($value)) || (int) $value <= 0) {
            throw new MarketplaceException('dependency_unavailable', 'The character service returned invalid data.');
        }

        return (int) $value;
    }

    /** @param array<string, mixed> $values */
    private function intValue(array $values, string $key): int
    {
        $value = $values[$key] ?? null;
        if (! is_int($value) && ! is_string($value)) {
            throw new MarketplaceException('dependency_unavailable', 'The character service returned invalid data.');
        }

        return (int) $value;
    }

    private function isTransientConcurrencyFailure(QueryException $exception): bool
    {
        return (string) $exception->getCode() === '40001'
            || $this->driverErrorCode($exception) === 1213;
    }

    private function driverErrorCode(QueryException $exception): ?int
    {
        $driverCode = $exception->errorInfo[1] ?? null;

        if (! is_int($driverCode) && ! is_string($driverCode)) {
            return null;
        }

        return (int) $driverCode;
    }
}
