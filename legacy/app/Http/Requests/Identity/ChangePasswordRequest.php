<?php

namespace App\Http\Requests\Identity;

use App\Identity\Support\IdentityPasswordPolicy;
use Illuminate\Foundation\Http\FormRequest;

final class ChangePasswordRequest extends FormRequest
{
    public function authorize(): bool
    {
        return true;
    }

    /**
     * @return array<string, array<int, mixed>>
     */
    public function rules(): array
    {
        return [
            'current_password' => [
                'required',
                'string',
                'max:1024',
                'current_password:web',
            ],
            'password' => [
                'required',
                'string',
                'max:1024',
                'confirmed',
                'different:current_password',
                IdentityPasswordPolicy::rule(),
            ],
        ];
    }

    public function newPassword(): string
    {
        $password = $this->validated('password');

        abort_unless(is_string($password), 422);

        return $password;
    }
}
