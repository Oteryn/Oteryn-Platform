<?php

namespace App\Http\Requests\CharacterProfiles;

use App\Identity\Models\Identity;
use Illuminate\Foundation\Http\FormRequest;

final class UpdateCharacterProfilePreferenceRequest extends FormRequest
{
    public function authorize(): bool
    {
        return $this->user() instanceof Identity;
    }

    /** @return array<string, list<string>> */
    public function rules(): array
    {
        return [
            'public_comment' => ['nullable', 'string', 'max:500'],
            'show_account_association' => ['sometimes', 'boolean'],
            'show_status' => ['sometimes', 'boolean'],
            'show_guild' => ['sometimes', 'boolean'],
            'show_house' => ['sometimes', 'boolean'],
            'show_skills' => ['sometimes', 'boolean'],
            'show_deaths' => ['sometimes', 'boolean'],
            'show_kills' => ['sometimes', 'boolean'],
            'is_main_character' => ['sometimes', 'boolean'],
        ];
    }

    /**
     * @return array{
     *     public_comment: string|null,
     *     show_account_association: bool,
     *     show_status: bool,
     *     show_guild: bool,
     *     show_house: bool,
     *     show_skills: bool,
     *     show_deaths: bool,
     *     show_kills: bool,
     *     is_main_character: bool
     * }
     */
    public function preferences(): array
    {
        $comment = preg_replace('/[[:cntrl:]]+/u', '', (string) $this->input('public_comment', ''));
        $comment = trim(is_string($comment) ? $comment : '');

        return [
            'public_comment' => $comment === '' ? null : $comment,
            'show_account_association' => $this->boolean('show_account_association'),
            'show_status' => $this->boolean('show_status'),
            'show_guild' => $this->boolean('show_guild'),
            'show_house' => $this->boolean('show_house'),
            'show_skills' => $this->boolean('show_skills'),
            'show_deaths' => $this->boolean('show_deaths'),
            'show_kills' => $this->boolean('show_kills'),
            'is_main_character' => $this->boolean('is_main_character'),
        ];
    }
}
