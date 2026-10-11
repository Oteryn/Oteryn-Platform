<?php

namespace App\Accounts\ReadModels;

use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersAccountView;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersReadModel;
use App\Identity\Models\Identity;
use Illuminate\Support\Collection;
use stdClass;

/**
 * Account overview over the native account-character read model keyed by the Platform AccountId.
 * The overview carries no game-account binding state.
 */
final class AccountOverviewReadModel
{
    public const STATE_READY = 'ready';

    public const CHARACTERS_AVAILABLE = 'available';

    public const CHARACTERS_EMPTY = 'empty';

    public const CHARACTERS_NOT_READY = 'not_ready';

    public const CHARACTERS_UNAVAILABLE = 'unavailable';

    public function __construct(private readonly NativeAccountCharactersReadModel $characters) {}

    /**
     * @return array{
     *     state: string,
     *     label: string,
     *     message: string,
     *     characters_state: string,
     *     characters_message: string,
     *     characters: Collection<int, stdClass>,
     *     character_count: int
     * }
     */
    public function forIdentity(Identity $identity): array
    {
        $view = $this->characters->viewForAccount($identity->account_id, now()->getTimestamp());

        $overview = [
            'state' => self::STATE_READY,
            'label' => 'Ready',
            'message' => 'Your game account is linked to your Platform account.',
        ];

        if ($view->state === NativeAccountCharactersAccountView::MISSING) {
            return [
                ...$overview,
                'characters_state' => self::CHARACTERS_NOT_READY,
                'characters_message' => 'Your characters will appear after the game servers report them.',
                'characters' => new Collection,
                'character_count' => 0,
            ];
        }

        if ($view->state !== NativeAccountCharactersAccountView::READY) {
            return [
                ...$overview,
                'characters_state' => self::CHARACTERS_UNAVAILABLE,
                'characters_message' => 'Your character list is temporarily unavailable.',
                'characters' => new Collection,
                'character_count' => 0,
            ];
        }

        $characters = (new Collection($view->characters))
            ->map(static function ($character): stdClass {
                $row = new stdClass;
                $row->name = $character->name;
                $row->availability = $character->availability;

                return $row;
            })
            ->values();
        $count = $characters->count();

        return [
            ...$overview,
            'characters_state' => $count === 0 ? self::CHARACTERS_EMPTY : self::CHARACTERS_AVAILABLE,
            'characters_message' => 'You do not have any active characters yet.',
            'characters' => $characters,
            'character_count' => $count,
        ];
    }
}
