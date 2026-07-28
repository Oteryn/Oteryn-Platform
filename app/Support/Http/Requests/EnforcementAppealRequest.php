<?php

namespace App\Support\Http\Requests;

use App\Identity\Models\Identity;
use Illuminate\Foundation\Http\FormRequest;

final class EnforcementAppealRequest extends FormRequest
{
    public function authorize(): bool
    {
        return $this->user() instanceof Identity;
    }

    /** @return array<string, mixed> */
    public function rules(): array
    {
        return [
            'appeal_message' => ['required', 'string', 'max:'.config('support.enforcement.appeal_max_length', 4000)],
            'lock_version' => ['required', 'integer', 'min:1'],
        ];
    }
}
