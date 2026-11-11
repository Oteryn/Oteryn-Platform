<?php

namespace App\Http\Requests\Identity;

use App\Identity\Models\Identity;
use Illuminate\Foundation\Http\FormRequest;
use Illuminate\Support\Facades\Hash;
use Illuminate\Validation\Validator;

final class ManageRecoveryKeyRequest extends FormRequest
{
    public function authorize(): bool
    {
        return $this->user() instanceof Identity;
    }

    /** @return array<string, list<string>> */
    public function rules(): array
    {
        return [
            'current_password' => ['required', 'string', 'max:1024'],
        ];
    }

    /** @return array<string, string> */
    public function messages(): array
    {
        return [
            'current_password.required' => __('identity.validation.password_required'),
            'current_password.string' => __('identity.validation.password_required'),
            'current_password.max' => __('identity.validation.password_too_long'),
        ];
    }

    public function withValidator(Validator $validator): void
    {
        $validator->after(function (Validator $validator): void {
            $identity = $this->user();
            $password = $this->input('current_password');
            if ($identity instanceof Identity && (! is_string($password) || ! Hash::check($password, $identity->password))) {
                $validator->errors()->add('current_password', __('identity.errors.current_password_invalid'));
            }
        });
    }
}
