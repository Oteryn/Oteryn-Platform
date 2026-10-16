<?php

namespace App\Http\Requests\Characters;

use Illuminate\Foundation\Http\FormRequest;
use Illuminate\Validation\Rule;

final class CreateCharacterRequest extends FormRequest
{
    public function authorize(): bool
    {
        return true;
    }

    /**
     * @return array<string, list<mixed>>
     */
    public function rules(): array
    {
        return [
            'name' => ['required', 'string', 'max:255'],
            'vocation' => ['required', 'integer', Rule::in([1, 2, 3, 4, 9])],
            'sex' => ['required', 'integer', Rule::in([0, 1])],
            // Only read when web creation is enabled; the action validates both strictly.
            'world' => ['nullable', 'string', 'max:36'],
            'submission' => ['nullable', 'string', 'max:36'],
        ];
    }

    public function characterName(): string
    {
        $name = $this->validated('name');

        return is_string($name) ? $name : '';
    }

    public function world(): ?string
    {
        $world = $this->validated('world');

        return is_string($world) ? $world : null;
    }

    public function submission(): ?string
    {
        $submission = $this->validated('submission');

        return is_string($submission) ? $submission : null;
    }

    public function vocation(): int
    {
        $vocation = $this->validated('vocation');

        return is_int($vocation) || is_string($vocation) ? (int) $vocation : -1;
    }

    public function sex(): int
    {
        $sex = $this->validated('sex');

        return is_int($sex) || is_string($sex) ? (int) $sex : -1;
    }
}
