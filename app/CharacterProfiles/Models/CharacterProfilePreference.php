<?php

namespace App\CharacterProfiles\Models;

use App\Identity\Models\Identity;
use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;

/**
 * @property int $id
 * @property int $identity_id
 * @property int $canary_player_id
 * @property string|null $public_comment
 * @property bool $show_account_association
 * @property bool $show_status
 * @property bool $show_guild
 * @property bool $show_house
 * @property bool $show_skills
 * @property bool $show_deaths
 * @property bool $show_kills
 * @property bool $is_main_character
 */
final class CharacterProfilePreference extends Model
{
    protected $fillable = [
        'identity_id',
        'canary_player_id',
        'public_comment',
        'show_account_association',
        'show_status',
        'show_guild',
        'show_house',
        'show_skills',
        'show_deaths',
        'show_kills',
        'is_main_character',
    ];

    /** @return BelongsTo<Identity, $this> */
    public function identity(): BelongsTo
    {
        return $this->belongsTo(Identity::class, 'identity_id');
    }

    /**
     * @return array<string, string>
     */
    protected function casts(): array
    {
        return [
            'identity_id' => 'integer',
            'canary_player_id' => 'integer',
            'show_account_association' => 'boolean',
            'show_status' => 'boolean',
            'show_guild' => 'boolean',
            'show_house' => 'boolean',
            'show_skills' => 'boolean',
            'show_deaths' => 'boolean',
            'show_kills' => 'boolean',
            'is_main_character' => 'boolean',
        ];
    }
}
