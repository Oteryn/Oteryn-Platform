<?php

namespace Tests\Unit\PublicGameData;

use App\PublicGameData\CharacterPresentation;
use Tests\TestCase;

final class CharacterPresentationTest extends TestCase
{
    public function test_known_base_and_promoted_vocations_have_readable_names(): void
    {
        $presentation = new CharacterPresentation;

        self::assertSame('Sorcerer', $presentation->vocationName(1));
        self::assertSame('Knight', $presentation->vocationName(4));
        self::assertSame('Monk', $presentation->vocationName(9));
        self::assertSame('Exalted Monk', $presentation->vocationName(10));
    }

    public function test_unknown_vocation_fails_visibly_without_exposing_an_empty_value(): void
    {
        $presentation = new CharacterPresentation;

        self::assertSame('Unknown vocation (ID 999)', $presentation->vocationName(999));
    }

    public function test_unknown_vocation_message_is_localized(): void
    {
        app()->setLocale('pl');
        $presentation = new CharacterPresentation;

        self::assertSame('Nieznana profesja (ID 999)', $presentation->vocationName(999));
    }
}
