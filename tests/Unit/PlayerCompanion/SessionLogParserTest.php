<?php

namespace Tests\Unit\PlayerCompanion;

use App\PlayerCompanion\SessionAnalysis\SessionLogParser;
use InvalidArgumentException;
use PHPUnit\Framework\Attributes\DataProvider;
use PHPUnit\Framework\TestCase;

final class SessionLogParserTest extends TestCase
{
    public function test_parses_summary_participants_rates_and_equal_split_settlement(): void
    {
        $result = (new SessionLogParser)->parse(<<<'LOG'
Session data: From 2026-08-13 18:00 to 2026-08-13 19:00
Session: 01:00h
XP Gain: 3,600,000
Loot: 600,000
Supplies: 200,000
Balance: 400,000
Damage: 5,000,000
Healing: 1,000,000
Alice
 Loot: 400,000
 Supplies: 100,000
 Balance: 300,000
 Damage: 3,000,000
 Healing: 600,000
Bob
 Loot: 200,000
 Supplies: 100,000
 Balance: 100,000
 Damage: 2,000,000
 Healing: 400,000
LOG);

        self::assertSame(3600, $result['session_seconds']);
        self::assertSame('1.1.0', $result['parser_version']);
        self::assertSame(3_600_000, $result['experience_gain']);
        self::assertSame(3_600_000, $result['experience_per_hour']);
        self::assertSame(400_000, $result['balance_value']);
        self::assertSame(400_000, $result['profit_per_hour']);
        self::assertSame(2, $result['participant_count']);
        self::assertSame('Alice', $result['settlements'][0]['from']);
        self::assertSame('Bob', $result['settlements'][0]['to']);
        self::assertSame(100_000, $result['settlements'][0]['amount']);
    }

    public function test_derives_totals_only_when_all_participants_supply_the_metric(): void
    {
        $result = (new SessionLogParser)->parse(<<<'LOG'
Session: 00:30h
Alice
Loot: 200,000
Supplies: 50,000
Damage: 100,000
Bob
Loot: 100,000
Supplies: 50,000
LOG);

        self::assertSame(300_000, $result['loot_value']);
        self::assertSame(100_000, $result['supplies_value']);
        self::assertSame(200_000, $result['balance_value']);
        self::assertSame(400_000, $result['profit_per_hour']);
        self::assertNull($result['damage']);
    }

    /** @return iterable<string, array{string}> */
    public static function invalidNumericValues(): iterable
    {
        yield 'decimal dot' => ['12.5'];
        yield 'decimal comma' => ['12,5'];
        yield 'mixed separators' => ['1,234.567'];
    }

    #[DataProvider('invalidNumericValues')]
    public function test_rejects_ambiguous_or_decimal_numeric_metrics(string $value): void
    {
        $this->expectException(InvalidArgumentException::class);
        $this->expectExceptionMessage('invalid_numeric_metric');

        (new SessionLogParser)->parse("Session: 01:00h\nLoot: {$value}");
    }

    public function test_rejects_duplicate_participant_names_case_insensitively(): void
    {
        $this->expectException(InvalidArgumentException::class);
        $this->expectExceptionMessage('duplicate_participant');

        (new SessionLogParser)->parse(<<<'LOG'
Session: 01:00h
Alice
Loot: 100
Supplies: 50
alice
Loot: 100
Supplies: 50
LOG);
    }

    public function test_rejects_more_than_twenty_participants(): void
    {
        $parts = ['Session: 01:00h'];
        for ($index = 0; $index < 21; $index++) {
            $parts[] = 'Player '.chr(65 + $index);
            $parts[] = 'Loot: 100';
            $parts[] = 'Supplies: 50';
        }

        $this->expectException(InvalidArgumentException::class);
        $this->expectExceptionMessage('too_many_participants');

        (new SessionLogParser)->parse(implode("\n", $parts));
    }

    public function test_rejects_log_without_supported_session_duration(): void
    {
        $this->expectException(InvalidArgumentException::class);
        $this->expectExceptionMessage('missing_session_duration');

        (new SessionLogParser)->parse("Loot: 100,000\nBalance: 50,000");
    }
}
