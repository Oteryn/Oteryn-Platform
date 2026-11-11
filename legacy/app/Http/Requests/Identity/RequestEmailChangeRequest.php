<?php

namespace App\Http\Requests\Identity;

use App\Identity\Models\Identity;
use App\Identity\Support\CanonicalEmail;
use Illuminate\Foundation\Http\FormRequest;
use Illuminate\Support\Facades\Hash;
use Illuminate\Validation\Validator;
use LogicException;

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

    /** @return array<string, string> */
    public function messages(): array
    {
        return [
            'email.required' => __('identity.validation.email_required'),
            'email.string' => __('identity.validation.email_invalid'),
            'email.email' => __('identity.validation.email_invalid'),
            'email.max' => __('identity.validation.email_too_long'),
            'email.confirmed' => __('identity.validation.email_confirmation'),
            'current_password.required' => __('identity.validation.password_required'),
            'current_password.string' => __('identity.validation.password_required'),
            'current_password.max' => __('identity.validation.password_too_long'),
        ];
    }

    public function withValidator(Validator $validator): void
    {
        $validator->after(function (Validator $validator): void {
            $identity = $this->user();
            if (! $identity instanceof Identity) {
                return;
            }

            $password = $this->input('current_password');
            if (! is_string($password) || ! Hash::check($password, $identity->password)) {
                $validator->errors()->add('current_password', __('identity.errors.current_password_invalid'));
            }

            $email = $this->input('email');
            if (is_string($email) && CanonicalEmail::normalize($email) === $identity->email) {
                $validator->errors()->add('email', __('identity.errors.email_must_differ'));
            }
        });
    }

    public function canonicalEmail(): string
    {
        $email = $this->validated('email');
        if (! is_string($email)) {
            throw new LogicException('The validated email change address is not a string.');
        }

        return CanonicalEmail::normalize($email);
    }
}
