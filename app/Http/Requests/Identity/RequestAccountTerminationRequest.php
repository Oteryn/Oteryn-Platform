<?php

namespace App\Http\Requests\Identity;

use App\Identity\Models\Identity;
use Illuminate\Foundation\Http\FormRequest;
use Illuminate\Support\Facades\Hash;
use Illuminate\Validation\Rule;
use Illuminate\Validation\Validator;

final class RequestAccountTerminationRequest extends FormRequest
{
    public function authorize(): bool
    {
        return $this->user() instanceof Identity;
    }

    /** @return array<string, list<mixed>> */
    public function rules(): array
    {
        return [
            'current_password' => ['required', 'string', 'max:1024'],
            'confirmation' => ['required', 'string', Rule::in([(string) config('identity_security.termination.confirmation_phrase', 'TERMINATE')])],
        ];
    }

    public function withValidator(Validator $validator): void
    {
        $validator->after(function (Validator $validator): void {
            $identity = $this->user();
            if ($identity instanceof Identity && ! Hash::check((string) $this->input('current_password'), $identity->password)) {
                $validator->errors()->add('current_password', 'The current password is invalid.');
            }
        });
    }
}
