<?php

namespace App\Characters\Policies;

use App\Characters\Exceptions\CharacterNameInvalid;

final class CharacterNamePolicy
{
    /** @var list<string> */
    private const RESERVED_WORDS = [
        'admin',
        'administrator',
        'god',
        'gamemaster',
        'gm',
        'cm',
        'support',
        'tutor',
        'moderator',
        'staff',
        'system',
        'server',
        'oteryn',
        'canary',
    ];

    /** @var list<string> */
    private const RESERVED_PHRASES = [
        'game master',
        'community manager',
    ];

    public function canonicalize(string $name): string
    {
        if ($name === '' || preg_match('/\A[A-Za-z ]+\z/', $name) !== 1) {
            throw new CharacterNameInvalid('Use only ASCII letters and spaces.');
        }

        $normalized = preg_replace('/ +/', ' ', trim($name, ' '));

        if (! is_string($normalized) || $normalized === '') {
            throw new CharacterNameInvalid('The character name is required.');
        }

        $words = explode(' ', $normalized);

        if (count($words) > 3) {
            throw new CharacterNameInvalid('Use between one and three words.');
        }

        $canonicalWords = [];

        foreach ($words as $word) {
            $length = strlen($word);

            if ($length < 2 || $length > 15) {
                throw new CharacterNameInvalid('Each word must contain between 2 and 15 letters.');
            }

            $canonicalWords[] = strtoupper($word[0]).strtolower(substr($word, 1));
        }

        $canonical = implode(' ', $canonicalWords);
        $canonicalLength = strlen($canonical);

        if ($canonicalLength < 3 || $canonicalLength > 29) {
            throw new CharacterNameInvalid('The canonical character name must contain between 3 and 29 characters.');
        }

        $lowerWords = array_map(static fn (string $word): string => strtolower($word), $canonicalWords);

        foreach ($lowerWords as $word) {
            if (in_array($word, self::RESERVED_WORDS, true)) {
                throw new CharacterNameInvalid('This character name is reserved.');
            }
        }

        $lowerCanonical = strtolower($canonical);

        if (in_array($lowerCanonical, self::RESERVED_PHRASES, true)) {
            throw new CharacterNameInvalid('This character name is reserved.');
        }

        $firstWord = $lowerWords[0];

        if (str_starts_with($firstWord, 'oteryn') || str_starts_with($firstWord, 'canary')) {
            throw new CharacterNameInvalid('This character name uses a protected product prefix.');
        }

        return $canonical;
    }
}
