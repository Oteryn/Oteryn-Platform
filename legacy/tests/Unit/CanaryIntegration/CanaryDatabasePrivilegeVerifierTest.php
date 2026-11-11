<?php

namespace Tests\Unit\CanaryIntegration;

use App\CanaryIntegration\CanaryDatabasePrivilegeVerifier;
use PHPUnit\Framework\TestCase;

class CanaryDatabasePrivilegeVerifierTest extends TestCase
{
    public function test_exact_direct_table_select_grants_are_accepted(): void
    {
        $violations = (new CanaryDatabasePrivilegeVerifier)->verify('canary', [
            "GRANT USAGE ON *.* TO `oteryn_readonly`@`%` IDENTIFIED BY PASSWORD '*REDACTED_TEST_HASH'",
            ...$this->requiredSelectGrants(),
        ]);

        $this->assertSame([], $violations);
    }

    public function test_write_privilege_is_rejected(): void
    {
        $violations = (new CanaryDatabasePrivilegeVerifier)->verify('canary', [
            'GRANT SELECT, INSERT ON `canary`.`players` TO `oteryn_readonly`@`%`',
            ...$this->remainingRequiredSelectGrants(except: 'players'),
        ]);

        $this->assertContains('Grant #1 includes a privilege other than direct SELECT.', $violations);
        $this->assertContains('Missing direct SELECT grant for required Canary table: players.', $violations);
    }

    public function test_schema_wide_select_is_rejected(): void
    {
        $violations = (new CanaryDatabasePrivilegeVerifier)->verify('canary', [
            'GRANT SELECT ON `canary`.* TO `oteryn_readonly`@`%`',
        ]);

        $this->assertContains('Grant #1 is global or schema-wide; only direct table SELECT grants are allowed.', $violations);
    }

    public function test_select_on_extra_table_is_rejected(): void
    {
        $violations = (new CanaryDatabasePrivilegeVerifier)->verify('canary', [
            ...$this->requiredSelectGrants(),
            'GRANT SELECT ON `canary`.`accounts` TO `oteryn_readonly`@`%`',
        ]);
        $extraGrantNumber = count(CanaryDatabasePrivilegeVerifier::REQUIRED_TABLES) + 1;

        $this->assertContains("Grant #{$extraGrantNumber} targets a table outside the approved Canary read allowlist.", $violations);
    }

    public function test_missing_player_deaths_grant_is_rejected(): void
    {
        $violations = (new CanaryDatabasePrivilegeVerifier)->verify(
            'canary',
            $this->remainingRequiredSelectGrants(except: 'player_deaths'),
        );

        $this->assertContains('Missing direct SELECT grant for required Canary table: player_deaths.', $violations);
    }

    public function test_role_based_grant_is_rejected_as_unverifiable(): void
    {
        $violations = (new CanaryDatabasePrivilegeVerifier)->verify('canary', [
            ...$this->requiredSelectGrants(),
            'GRANT `canary_reader`@`%` TO `oteryn_readonly`@`%`',
        ]);
        $roleGrantNumber = count(CanaryDatabasePrivilegeVerifier::REQUIRED_TABLES) + 1;

        $this->assertContains("Grant #{$roleGrantNumber} has an unsupported grant shape.", $violations);
    }

    public function test_grant_option_is_rejected(): void
    {
        $violations = (new CanaryDatabasePrivilegeVerifier)->verify('canary', [
            'GRANT SELECT ON `canary`.`players` TO `oteryn_readonly`@`%` WITH GRANT OPTION',
            ...$this->remainingRequiredSelectGrants(except: 'players'),
        ]);

        $this->assertContains('Grant #1 includes GRANT OPTION.', $violations);
        $this->assertContains('Missing direct SELECT grant for required Canary table: players.', $violations);
    }

    /** @return list<string> */
    private function requiredSelectGrants(): array
    {
        return array_map(
            static fn (string $table): string => "GRANT SELECT ON `canary`.`{$table}` TO `oteryn_readonly`@`%`",
            CanaryDatabasePrivilegeVerifier::REQUIRED_TABLES,
        );
    }

    /** @return list<string> */
    private function remainingRequiredSelectGrants(string $except): array
    {
        return array_values(array_filter(
            $this->requiredSelectGrants(),
            static fn (string $grant): bool => ! str_contains($grant, ".`{$except}` "),
        ));
    }
}
