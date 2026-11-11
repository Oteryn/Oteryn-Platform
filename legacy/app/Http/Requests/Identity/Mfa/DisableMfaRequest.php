<?php

namespace App\Http\Requests\Identity\Mfa;

use Illuminate\Foundation\Http\FormRequest;

final class DisableMfaRequest extends FormRequest
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
            'current_password' => [
                'required',
                'string',
                'max:1024',
            ],
            'code' => [
                'required',
                'string',
                'max:64',
            ],
        ];
    }

    public function currentPassword(): string
    {
        $value = $this->validated('current_password');
        abort_unless(is_string($value), 422);

        return $value;
    }

    public function code(): string
    {
        $value = $this->validated('code');
        abort_unless(is_string($value), 422);

        return $value;
    }
}
