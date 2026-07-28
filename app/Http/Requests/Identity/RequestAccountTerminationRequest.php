<?php

namespace App\Http\Requests\Identity;

use App\Identity\Models\Identity;
use Illuminate\Foundation\Http\FormRequest;
use Illuminate\Support\Facades\Hash;
use Illuminate\Validation\Rule;
use Illuminate\Validation\Validator;
use LogicException;

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
            'confirmation' => ['required', 'string', Rule::in([$this->confirmationPhrase()])],
        ];
    }

    public function withValidator(Validator $validator): void
    {
        $validator->after(function (Validator $validator): void {
            $identity = $this->user();
            $password = $this->input('current_password');
            if ($identity instanceof Identity && (! is_string($password) || ! Hash::check($password, $identity->password))) {
                $validator->errors()->add('current_password', 'The current password is invalid.');
            }
        });
    }

    private function confirmationPhrase(): string
    {
        $phrase = config('identity_security.termination.confirmation_phrase');
        if (! is_string($phrase) || $phrase === '') {
            throw new LogicException('The account termination confirmation phrase is invalid.');
        }

        return $phrase;
    }
}
