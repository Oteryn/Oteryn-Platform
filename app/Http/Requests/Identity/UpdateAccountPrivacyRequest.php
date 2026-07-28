<?php

namespace App\Http\Requests\Identity;

use App\Identity\Models\Identity;
use Illuminate\Foundation\Http\FormRequest;

final class UpdateAccountPrivacyRequest extends FormRequest
{
    public function authorize(): bool
    {
        return $this->user() instanceof Identity;
    }

    /** @return array<string, list<string>> */
    public function rules(): array
    {
        return [
            'public_account_association' => ['sometimes', 'boolean'],
            'public_status_visible' => ['sometimes', 'boolean'],
        ];
    }

    /** @return array{public_account_association: bool, public_status_visible: bool} */
    public function privacy(): array
    {
        return [
            'public_account_association' => $this->boolean('public_account_association'),
            'public_status_visible' => $this->boolean('public_status_visible'),
        ];
    }
}
