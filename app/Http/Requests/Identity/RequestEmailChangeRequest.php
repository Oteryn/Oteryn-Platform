<?php

namespace App\Http\Requests\Identity;

use App\Identity\Models\Identity;
use App\Identity\Support\CanonicalEmail;
use Illuminate\Foundation\Http\FormRequest;
use Illuminate\Support\Facades\Hash;
use Illuminate\Validation\Validator;

final class RequestEmailChangeRequest extends FormRequest
{
    public function authorize(): bool
    {
        return $this->user() instanceof Identity;
    }

    /** @return array<string, list<string>> */
    public function rules(): array
    {
        return [
            'email' => ['required', 'string', 'email:rfc', 'max:254', 'confirmed'],
            'current_password' => ['required', 'string', 'max:1024'],
        ];
    }

    public function withValidator(Validator $validator): void
    {
        $validator->after(function (Validator $validator): void {
            $identity = $this->user();
            if (! $identity instanceof Identity) {
                return;
            }

            if (! Hash::check((string) $this->input('current_password'), $identity->password)) {
                $validator->errors()->add('current_password', 'The current password is invalid.');
            }

            $email = $this->input('email');
            if (is_string($email) && CanonicalEmail::normalize($email) === $identity->email) {
                $validator->errors()->add('email', 'The new email address must be different.');
            }
        });
    }

    public function canonicalEmail(): string
    {
        return CanonicalEmail::normalize((string) $this->validated('email'));
    }
}
