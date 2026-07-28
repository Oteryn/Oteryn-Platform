<?php

namespace App\Support\Http\Requests;

use App\Identity\Models\Identity;
use App\Support\Models\EnforcementRecord;
use Illuminate\Foundation\Http\FormRequest;
use Illuminate\Validation\Rule;

final class AdminEnforcementAppealRequest extends FormRequest
{
    public function authorize(): bool
    {
        return $this->user() instanceof Identity;
    }

    /** @return array<string, mixed> */
    public function rules(): array
    {
        return [
            'appeal_status' => ['required', 'string', Rule::in([
                EnforcementRecord::APPEAL_REVIEWING,
                EnforcementRecord::APPEAL_ACCEPTED,
                EnforcementRecord::APPEAL_REJECTED,
            ])],
            'appeal_outcome' => ['nullable', 'string', 'max:'.config('support.enforcement.appeal_max_length', 4000)],
            'lock_version' => ['required', 'integer', 'min:1'],
        ];
    }
}
