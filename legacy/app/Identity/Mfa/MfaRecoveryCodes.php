<?php

namespace App\Identity\Mfa;

use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Str;

final class MfaRecoveryCodes
{
    private const CODE_COUNT = 8;

    private const RAW_CODE_LENGTH = 10;

    /**
     * @return array{plain: list<string>, hashes: list<string>}
     */
    public function generate(): array
    {
        $plain = [];
        $hashes = [];

        for ($index = 0; $index < self::CODE_COUNT; $index++) {
            $normalized = strtoupper(Str::random(self::RAW_CODE_LENGTH));
            $plain[] = substr($normalized, 0, 5).'-'.substr($normalized, 5);
            $hashes[] = Hash::make($normalized);
        }

        return [
            'plain' => $plain,
            'hashes' => $hashes,
        ];
    }

    public function normalize(string $code): string
    {
        $normalized = preg_replace('/[^A-Za-z0-9]/', '', $code);

        return strtoupper(is_string($normalized) ? $normalized : '');
    }
}
