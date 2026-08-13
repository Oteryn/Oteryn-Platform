<?php

namespace App\PlayerCompanion\SessionAnalysis;

use InvalidArgumentException;

final class SessionLogParser
{
    public const SOURCE_FORMAT = 'tibia-session-text-v1';

    public const PARSER_VERSION = '1.1.0';

    public const FORMULA_VERSION = 'equal-split-v1';

    private const MAX_METRIC = 9_000_000_000_000_000;

    private const MAX_PARTICIPANTS = 20;

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
        $sessionSeconds = null;
        $experience = null;
        $loot = null;
        $supplies = null;
        $balance = null;
        $damage = null;
        $healing = null;
        $participants = [];
        $participantNames = [];
        $participantIndex = null;

        foreach ($lines as $index => $line) {
            $trimmed = trim($line);
            if ($trimmed === '') {
                continue;
            }

            if ($this->isParticipantHeading($lines, $index, $trimmed)) {
                $normalizedName = mb_strtolower($trimmed);
                if (isset($participantNames[$normalizedName])) {
                    throw new InvalidArgumentException('duplicate_participant');
                }
                if (count($participants) >= self::MAX_PARTICIPANTS) {
                    throw new InvalidArgumentException('too_many_participants');
                }

                $participantNames[$normalizedName] = true;
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
                $sessionSeconds ??= $this->parseDuration($value);
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
                $participants[$participantIndex] = $this->withParticipantMetric(
                    $participants[$participantIndex],
                    $field,
                    $number,
                );

                continue;
            }

            match ($field) {
                'experience_gain' => $experience ??= $number,
                'loot_value' => $loot ??= $number,
                'supplies_value' => $supplies ??= $number,
                'balance_value' => $balance ??= $number,
                'damage' => $damage ??= $number,
                'healing' => $healing ??= $number,
            };
        }

        if (! is_int($sessionSeconds) || $sessionSeconds <= 0) {
            throw new InvalidArgumentException('missing_session_duration');
        }

        foreach ($participants as $index => $participant) {
            if ($participant['balance_value'] === null && $participant['loot_value'] !== null && $participant['supplies_value'] !== null) {
                $participants[$index] = $this->withParticipantMetric(
                    $participant,
                    'balance_value',
                    $participant['loot_value'] - $participant['supplies_value'],
                );
            }
        }

        $loot ??= $this->completeParticipantMetric($participants, 'loot_value');
        $supplies ??= $this->completeParticipantMetric($participants, 'supplies_value');
        if ($balance === null && $loot !== null && $supplies !== null) {
            $balance = $loot - $supplies;
        }
        if ($balance === null) {
            $balance = $this->completeParticipantMetric($participants, 'balance_value');
        }

        $damage ??= $this->completeParticipantMetric($participants, 'damage');
        $healing ??= $this->completeParticipantMetric($participants, 'healing');

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
        $normalized = mb_strtolower($candidate);
        if (
            str_contains($candidate, ':')
            || mb_strlen($candidate) > 40
            || preg_match("/^\\p{L}[\\p{L} '\\-]{0,39}$/u", $candidate) !== 1
            || in_array($normalized, ['party hunt analyzer', 'hunt analyzer', 'session', 'session data'], true)
        ) {
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
            $minutes = ($matches[2] ?? '') !== '' ? (int) $matches[2] : 0;
            if ($minutes >= 60 || ($hours === 0 && $minutes === 0)) {
                throw new InvalidArgumentException('invalid_session_duration');
            }

            return ($hours * 3600) + ($minutes * 60);
        }

        throw new InvalidArgumentException('invalid_session_duration');
    }

    private function parseInteger(string $value): int
    {
        $value = trim($value);
        if (preg_match('/^-?\d+$/', $value) === 1) {
            $normalized = $value;
        } elseif (
            preg_match('/^-?\d{1,3}(?:,\d{3})+$/', $value) === 1
            || preg_match('/^-?\d{1,3}(?:\.\d{3})+$/', $value) === 1
            || preg_match('/^-?\d{1,3}(?: \d{3})+$/', $value) === 1
        ) {
            $normalized = str_replace([',', '.', ' '], '', $value);
        } else {
            throw new InvalidArgumentException('invalid_numeric_metric');
        }

        $number = filter_var($normalized, FILTER_VALIDATE_INT);
        if (! is_int($number) || abs($number) > self::MAX_METRIC) {
            throw new InvalidArgumentException('metric_out_of_range');
        }

        return $number;
    }

    /**
     * Return a participant-derived aggregate only when every participant supplied the metric.
     * Partial values remain unknown instead of being presented as a complete total.
     *
     * @param  list<array{name:string,loot_value:int|null,supplies_value:int|null,balance_value:int|null,damage:int|null,healing:int|null}>  $participants
     */
    private function completeParticipantMetric(array $participants, string $field): ?int
    {
        if ($participants === []) {
            return null;
        }

        $sum = 0;
        foreach ($participants as $participant) {
            $value = $participant[$field] ?? null;
            if (! is_int($value)) {
                return null;
            }
            $sum += $value;
        }

        return $sum;
    }

    private function perHour(?int $value, int $seconds): ?int
    {
        if ($value === null) {
            return null;
        }

        return (int) round(($value * 3600) / $seconds);
    }

    /**
     * @param  array{name:string,loot_value:int|null,supplies_value:int|null,balance_value:int|null,damage:int|null,healing:int|null}  $participant
     * @param  'loot_value'|'supplies_value'|'balance_value'|'damage'|'healing'  $field
     * @return array{name:string,loot_value:int|null,supplies_value:int|null,balance_value:int|null,damage:int|null,healing:int|null}
     */
    private function withParticipantMetric(array $participant, string $field, int $value): array
    {
        return match ($field) {
            'loot_value' => [...$participant, 'loot_value' => $value],
            'supplies_value' => [...$participant, 'supplies_value' => $value],
            'balance_value' => [...$participant, 'balance_value' => $value],
            'damage' => [...$participant, 'damage' => $value],
            'healing' => [...$participant, 'healing' => $value],
        };
    }

    /**
     * @param  list<array{name:string,loot_value:int|null,supplies_value:int|null,balance_value:int|null,damage:int|null,healing:int|null}>  $participants
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
            $amount = min(abs($payers[$payerIndex]['delta']), $receivers[$receiverIndex]['delta']);
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
