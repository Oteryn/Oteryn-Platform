<?php

namespace App\GameCatalog\Infrastructure\Json;

use JsonException;
use RuntimeException;

final class DuplicateJsonKeyDetector
{
    private int $offset = 0;

    private int $length = 0;

    /** @var list<string> */
    private array $duplicates = [];

    public function __construct(private readonly int $maximumDepth = 128) {}

    /**
     * @return list<string>
     */
    public function find(string $json): array
    {
        $this->offset = 0;
        $this->length = strlen($json);
        $this->duplicates = [];

        $this->parseValue($json, '$', 0);
        $this->skipWhitespace($json);

        if ($this->offset !== $this->length) {
            throw new RuntimeException('Unexpected trailing JSON content.');
        }

        return $this->duplicates;
    }

    private function parseValue(string $json, string $path, int $depth): void
    {
        if ($depth > $this->maximumDepth) {
            throw new RuntimeException('JSON nesting exceeds the configured maximum depth.');
        }

        $this->skipWhitespace($json);
        $character = $json[$this->offset] ?? null;

        match ($character) {
            '{' => $this->parseObject($json, $path, $depth + 1),
            '[' => $this->parseArray($json, $path, $depth + 1),
            '"' => $this->parseString($json),
            't' => $this->parseLiteral($json, 'true'),
            'f' => $this->parseLiteral($json, 'false'),
            'n' => $this->parseLiteral($json, 'null'),
            default => $this->parseNumber($json),
        };
    }

    private function parseObject(string $json, string $path, int $depth): void
    {
        $this->offset++;
        $this->skipWhitespace($json);

        /** @var array<string, true> $keys */
        $keys = [];
        if (($json[$this->offset] ?? null) === '}') {
            $this->offset++;

            return;
        }

        while (true) {
            $this->skipWhitespace($json);
            $key = $this->parseString($json);
            $keyPath = $path.'.'.$key;

            if (isset($keys[$key])) {
                $this->duplicates[] = $keyPath;
            }
            $keys[$key] = true;

            $this->skipWhitespace($json);
            $this->expect($json, ':');
            $this->parseValue($json, $keyPath, $depth);
            $this->skipWhitespace($json);

            $character = $json[$this->offset] ?? null;
            if ($character === '}') {
                $this->offset++;

                return;
            }

            $this->expect($json, ',');
        }
    }

    private function parseArray(string $json, string $path, int $depth): void
    {
        $this->offset++;
        $this->skipWhitespace($json);

        if (($json[$this->offset] ?? null) === ']') {
            $this->offset++;

            return;
        }

        $index = 0;
        while (true) {
            $this->parseValue($json, $path.'['.$index.']', $depth);
            $index++;
            $this->skipWhitespace($json);

            $character = $json[$this->offset] ?? null;
            if ($character === ']') {
                $this->offset++;

                return;
            }

            $this->expect($json, ',');
        }
    }

    private function parseString(string $json): string
    {
        $start = $this->offset;
        $this->expect($json, '"');

        while ($this->offset < $this->length) {
            $character = $json[$this->offset++];
            if ($character === '"') {
                $encoded = substr($json, $start, $this->offset - $start);

                try {
                    $decoded = json_decode($encoded, true, 8, JSON_THROW_ON_ERROR);
                } catch (JsonException $exception) {
                    throw new RuntimeException('Invalid JSON string.', previous: $exception);
                }

                if (! is_string($decoded)) {
                    throw new RuntimeException('JSON object key did not decode to a string.');
                }

                return $decoded;
            }

            if ($character === '\\') {
                $escaped = $json[$this->offset++] ?? null;
                if ($escaped === 'u') {
                    if ($this->offset + 4 > $this->length) {
                        throw new RuntimeException('Invalid JSON unicode escape.');
                    }
                    $this->offset += 4;
                }
            }
        }

        throw new RuntimeException('Unterminated JSON string.');
    }

    private function parseNumber(string $json): void
    {
        if (preg_match('/\G-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?/', $json, $matches, 0, $this->offset) !== 1) {
            throw new RuntimeException('Invalid JSON scalar.');
        }

        $this->offset += strlen($matches[0]);
    }

    private function parseLiteral(string $json, string $literal): void
    {
        if (substr($json, $this->offset, strlen($literal)) !== $literal) {
            throw new RuntimeException('Invalid JSON literal.');
        }

        $this->offset += strlen($literal);
    }

    private function skipWhitespace(string $json): void
    {
        while ($this->offset < $this->length && str_contains(" \t\r\n", $json[$this->offset])) {
            $this->offset++;
        }
    }

    private function expect(string $json, string $character): void
    {
        if (($json[$this->offset] ?? null) !== $character) {
            throw new RuntimeException("Expected JSON token {$character}.");
        }

        $this->offset++;
    }
}
