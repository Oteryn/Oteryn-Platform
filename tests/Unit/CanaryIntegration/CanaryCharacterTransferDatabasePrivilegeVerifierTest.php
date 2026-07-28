<?php

namespace Tests\Unit\CanaryIntegration;

use App\CanaryIntegration\CanaryCharacterTransfer;
use App\CanaryIntegration\CanaryCharacterTransferDatabasePrivilegeVerifier;
use PHPUnit\Framework\TestCase;

final class CanaryCharacterTransferDatabasePrivilegeVerifierTest extends TestCase
{
    public function test_exact_column_level_grants_are_accepted(): void
    {
        $violations = (new CanaryCharacterTransferDatabasePrivilegeVerifier)->verify('canary', $this->validGrants());

        self::assertSame([], $violations);
    }

    public function test_table_level_and_unrelated_privileges_are_rejected(): void
    {
        $grants = $this->validGrants();
        $grants[] = "GRANT SELECT ON `canary`.`players` TO 'oteryn_character_transfer'@'%'";
        $grants[] = "GRANT SELECT (`password`) ON `canary`.`accounts` TO 'oteryn_character_transfer'@'%'";
        $grants[] = "GRANT DELETE ON `canary`.`players` TO 'oteryn_character_transfer'@'%'";

        $violations = (new CanaryCharacterTransferDatabasePrivilegeVerifier)->verify('canary', $grants);

        self::assertNotSame([], $violations);
    }

    public function test_extra_update_column_and_grant_option_are_rejected(): void
    {
        $grants = $this->validGrants();
        $grants[] = "GRANT UPDATE (`name`) ON `canary`.`players` TO 'oteryn_character_transfer'@'%'";
        $grants[] = "GRANT SELECT (`id`) ON `canary`.`accounts` TO 'oteryn_character_transfer'@'%' WITH GRANT OPTION";

        $violations = (new CanaryCharacterTransferDatabasePrivilegeVerifier)->verify('canary', $grants);

        self::assertNotSame([], $violations);
    }

    public function test_missing_session_field_is_rejected(): void
    {
        $grants = $this->validGrants();
        $grants[3] = str_replace(', `expires_at`', '', $grants[3]);

        $violations = (new CanaryCharacterTransferDatabasePrivilegeVerifier)->verify('canary', $grants);

        self::assertContains('Missing approved SELECT privilege for cluster_sessions.expires_at.', $violations);
    }

    /** @return list<string> */
    private function validGrants(): array
    {
        $playerColumns = implode(', ', array_map(
            static fn (string $column): string => "`{$column}`",
            CanaryCharacterTransfer::PLAYER_SELECT_COLUMNS,
        ));

        return [
            "GRANT USAGE ON *.* TO 'oteryn_character_transfer'@'%'",
            "GRANT SELECT (`id`) ON `canary`.`accounts` TO 'oteryn_character_transfer'@'%'",
            "GRANT SELECT ({$playerColumns}), UPDATE (`account_id`) ON `canary`.`players` TO 'oteryn_character_transfer'@'%'",
            "GRANT SELECT (`player_id`, `account_id`, `status`, `expires_at`) ON `canary`.`cluster_sessions` TO 'oteryn_character_transfer'@'%'",
        ];
    }
}
