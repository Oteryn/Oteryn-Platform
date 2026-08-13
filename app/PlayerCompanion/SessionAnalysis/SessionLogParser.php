<?php

namespace App\PlayerCompanion\SessionAnalysis;

use InvalidArgumentException;

final class SessionLogParser
{
    public const SOURCE_FORMAT = 'tibia-session-text-v1';

    public const PARSER_VERSION = '1.0.0';

    public const FORMULA_VERSION = 'equal-split-v1';

    private const MAX_METRIC = 9_000_000_000_000_000;

    /**
     * Parse a bounded Tibia-style session/party-hunt text export into normalized private data.
     *
     * @return array{
     *   source_format:string,
     *   parser_version:string,
     *   formula_version:string,
     *   session_seconds:int,
     *   experience_gain:int|null,
     *   loot_value:int|null,
     *   supplies_value:int|null,
     *   balance_value:int|null,
     *   damage:int|null,
     *   healing:int|null,
     *   experience_per_hour:int|null,
     *   profit_per_hour:int|null,
     *   participant_count:int,
     *   participants:list<array{name:string,loot_value:int|null,supplies_value:int|null,balance_value:int|null,damage:int|null,healing:int|null}>,
     *   settlements:list<array{from:string,to:string,amount:int}>
     * }
     */
    public function parse(string $raw): array
    {
        $text = trim(str_replace(["\r\n", "\r"], "\n", $raw));
        if ($text === '') {
            throw new InvalidArgumentException('empty_log');
        }

        $lines = explode("\n", $text);
        $summary = [];
        $participants = [];
        $participantIndex = null;

        foreach ($lines as $index => $line) {
            $trimmed = trim($line);
            if ($trimmed === '') {
                continue;
            }

            if ($this->isParticipantHeading($lines, $index, $trimmed)) {
                $participantIndex = count($participants);
                $participants[] = [
                    'name' => $trimmed,
                    'loot_value' => null,
                    'supplies_value' => null,
                    'balance_value' => null,
                    'damage' => null,
                    'healing' => null,
                ];
                continue;
            }

            if (! preg_match('/^(Session|XP Gain|Loot|Supplies|Balance|Damage|Healing)\s*:\s*(.+)$/i', $trimmed, $matches)) {
                continue;
            }

            $key = strtolower($matches[1]);
            $value = trim($matches[2]);

            if ($key === 'session') {
                $summary['session_seconds'] ??= $this->parseDuration($value);
                $participantIndex = null;
                continue;
            }

            $field = match ($key) {
                'xp gain' => 'experience_gain',
                'loot' => 'loot_value',
                'supplies' => 'supplies_value',
                'balance' => 'balance_value',
                'damage' => 'damage',
                'healing' => 'healing',
                default => null,
            };

            if ($field === null) {
                continue;
            }

            $number = $this->parseInteger($value);

            if ($participantIndex !== null && $field !== 'experience_gain') {
                $participants[$participantIndex][$field] = $number;
                continue;
            }

            $summary[$field] ??= $number;
        }

        $sessionSeconds = $summary['session_seconds'] ?? null;
        if (! is_int($sessionSeconds) || $sessionSeconds <= 0) {
            throw new InvalidArgumentException('missing_session_duration');
        }

        foreach ($participants as &$participant) {
            if ($participant['balance_value'] === null && $participant['loot_value'] !== null && $participant['supplies_value'] !== null) {
                $participant['balance_value'] = $participant['loot_value'] - $participant['supplies_value'];
            }
        }
        unset($participant);

        $loot = $summary['loot_value'] ?? $this->sumParticipantMetric($participants, 'loot_value');
        $supplies = $summary['supplies_value'] ?? $this->sumParticipantMetric($participants, 'supplies_value');
        $balance = $summary['balance_value'] ?? null;
        if ($balance === null && $loot !== null && $supplies !== null) {
            $balance = $loot - $supplies;
        }
        if ($balance === null) {
            $balance = $this->sumParticipantMetric($participants, 'balance_value');
        }

        $experience = $summary['experience_gain'] ?? null;
        $damage = $summary['damage'] ?? $this->sumParticipantMetric($participants, 'damage');
        $healing = $summary['healing'] ?? $this->sumParticipantMetric($participants, 'healing');

        if ($experience === null && $loot === null && $supplies === null && $balance === null && $participants === []) {
            throw new InvalidArgumentException('missing_session_metrics');
        }

        return [
            'source_format' => self::SOURCE_FORMAT,
            'parser_version' => self::PARSER_VERSION,
            'formula_version' => self::FORMULA_VERSION,
            'session_seconds' => $sessionSeconds,
            'experience_gain' => $experience,
            'loot_value' => $loot,
            'supplies_value' => $supplies,
            'balance_value' => $balance,
            'damage' => $damage,
            'healing' => $healing,
            'experience_per_hour' => $this->perHour($experience, $sessionSeconds),
            'profit_per_hour' => $this->perHour($balance, $sessionSeconds),
            'participant_count' => count($participants),
            'participants' => $participants,
            'settlements' => $this->settlements($participants),
        ];
    }

    /** @param list<string> $lines */
    private function isParticipantHeading(array $lines, int $index, string $candidate): bool
    {
        if (str_contains($candidate, ':') || mb_strlen($candidate) > 40) {
            return false;
        }

        for ($next = $index + 1, $count = count($lines); $next < $count; $next++) {
            $line = trim($lines[$next]);
            if ($line === '') {
                continue;
            }

            return preg_match('/^(Loot|Supplies|Balance|Damage|Healing)\s*:/i', $line) === 1;
        }

        return false;
    }

    private function parseDuration(string $value): int
    {
        if (preg_match('/^(\d{1,3}):(\d{2})h?$/i', trim($value), $matches)) {
            $minutes = (int) $matches[2];
            if ($minutes >= 60) {
                throw new InvalidArgumentException('invalid_session_duration');
            }

            return ((int) $matches[1] * 3600) + ($minutes * 60);
        }

        if (preg_match('/^(?:(\d{1,3})h\s*)?(?:(\d{1,2})m)?$/i', trim($value), $matches)) {
            $hours = isset($matches[1]) && $matches[1] !== '' ? (int) $matches[1] : 0;
            $minutes = isset($matches[2]) && $matches[2] !== '' ? (int) $matches[2] : 0;
            if ($minutes >= 60 || ($hours === 0 && $minutes === 0)) {
                throw new InvalidArgumentException('invalid_session_duration');
            }

            return ($hours * 3600) + ($minutes * 60);
        }

        throw new InvalidArgumentException('invalid_session_duration');
    }

    private function parseInteger(string $value): int
    {
        $normalized = preg_replace('/[.,\s]/', '', trim($value));
        if (! is_string($normalized) || preg_match('/^-?\d+$/', $normalized) !== 1) {
            throw new InvalidArgumentException('invalid_numeric_metric');
        }

        $number = filter_var($normalized, FILTER_VALIDATE_INT);
        if (! is_int($number) || abs($number) > self::MAX_METRIC) {
            throw new InvalidArgumentException('metric_out_of_range');
        }

        return $number;
    }

    /**
     * @param list<array{name:string,loot_value:int|null,supplies_value:int|null,balance_value:int|null,damage:int|null,healing:int|null}> $participants
     */
    private function sumParticipantMetric(array $participants, string $field): ?int
    {
        if ($participants === []) {
            return null;
        }

        $sum = 0;
        $found = false;
        foreach ($participants as $participant) {
            $value = $participant[$field] ?? null;
            if (is_int($value)) {
                $sum += $value;
                $found = true;
            }
        }

        return $found ? $sum : null;
    }

    private function perHour(?int $value, int $seconds): ?int
    {
        if ($value === null) {
            return null;
        }

        return (int) round(($value * 3600) / $seconds);
    }

    /**
     * @param list<array{name:string,loot_value:int|null,supplies_value:int|null,balance_value:int|null,damage:int|null,healing:int|null}> $participants
     * @return list<array{from:string,to:string,amount:int}>
     */
    private function settlements(array $participants): array
    {
        if (count($participants) < 2) {
            return [];
        }

        foreach ($participants as $participant) {
            if (! is_int($participant['balance_value'])) {
                return [];
            }
        }

        $total = array_sum(array_column($participants, 'balance_value'));
        $count = count($participants);
        $baseTarget = intdiv($total, $count);
        $remainder = $total - ($baseTarget * $count);
        $deltas = [];

        foreach ($participants as $index => $participant) {
            $target = $baseTarget;
            if ($remainder > 0 && $index < $remainder) {
                $target++;
            } elseif ($remainder < 0 && $index < abs($remainder)) {
                $target--;
            }

            $deltas[] = [
                'name' => $participant['name'],
                'delta' => $target - $participant['balance_value'],
            ];
        }

        $payers = array_values(array_filter($deltas, static fn (array $row): bool => $row['delta'] < 0));
        $receivers = array_values(array_filter($deltas, static fn (array $row): bool => $row['delta'] > 0));
        $settlements = [];
        $payerIndex = 0;
        $receiverIndex = 0;

        while (isset($payers[$payerIndex], $receivers[$receiverIndex])) {
            $amount = min(-$payers[$payerIndex]['delta'], $receivers[$receiverIndex]['delta']);
            if ($amount > 0) {
                $settlements[] = [
                    'from' => $payers[$payerIndex]['name'],
                    'to' => $receivers[$receiverIndex]['name'],
                    'amount' => $amount,
                ];
            }

            $payers[$payerIndex]['delta'] += $amount;
            $receivers[$receiverIndex]['delta'] -= $amount;

            if ($payers[$payerIndex]['delta'] === 0) {
                $payerIndex++;
            }
            if ($receivers[$receiverIndex]['delta'] === 0) {
                $receiverIndex++;
            }
        }

        return $settlements;
    }
}
