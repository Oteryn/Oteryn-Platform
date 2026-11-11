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
            'show_guild' => ['sometimes', 'boolean'],
            'is_main_character' => ['sometimes', 'boolean'],
        ];
    }

    /**
     * Only what the native public page can honour: the guild row (an upper bound over Game's own
     * flag) and the main-character mark of the account overview.
     *
     * @return array{show_guild: bool, is_main_character: bool}
     */
    public function preferences(): array
    {
        return [
            'show_guild' => $this->boolean('show_guild'),
            'is_main_character' => $this->boolean('is_main_character'),
        ];
    }
}
