<?php

namespace App\Http\Requests\Identity\Mfa;

use Illuminate\Foundation\Http\FormRequest;

final class MfaChallengeRequest extends FormRequest
{
    public function authorize(): bool
    {
        return true;
    }

    /**
     * @return array<string, array<int, string>>
     */
    public function rules(): array
    {
        return [
            'code' => [
                'required',
                'string',
                'max:64',
            ],
        ];
    }

    public function code(): string
    {
        $value = $this->validated('code');
        abort_unless(is_string($value), 422);

        return $value;
    }
}
