<?php

namespace App\Support\Http\Requests;

use App\Identity\Models\Identity;
use App\Support\Models\EnforcementRecord;
use App\Support\SupportConfiguration;
use Illuminate\Foundation\Http\FormRequest;
use Illuminate\Validation\Rule;

final class EnforcementRecordRequest extends FormRequest
{
    public function authorize(): bool
    {
        return $this->user() instanceof Identity;
    }

    /** @return array<string, mixed> */
    public function rules(): array
    {
        return [
            'target_identity_id' => ['required', 'integer', 'exists:identities,id'],
            'category' => ['required', 'string', Rule::in(EnforcementRecord::categories())],
            'status' => ['required', 'string', Rule::in(EnforcementRecord::statuses())],
            'public_reason' => ['required', 'string', 'max:'.SupportConfiguration::positiveInteger('support.enforcement.reason_max_length', 4000)],
            'moderator_notes' => ['nullable', 'string', 'max:8000'],
            'effective_at' => ['required', 'date'],
            'expires_at' => ['nullable', 'date', 'after:effective_at'],
            'lock_version' => [$this->isMethod('POST') ? 'nullable' : 'required', 'integer', 'min:1'],
        ];
    }
}
