<?php

namespace App\Support\Http\Requests;

use App\Identity\Models\Identity;
use App\Support\Models\PlayerReport;
use Illuminate\Foundation\Http\FormRequest;
use Illuminate\Validation\Rule;

final class AdminReportModerationRequest extends FormRequest
{
    public function authorize(): bool
    {
        return $this->user() instanceof Identity;
    }

    /** @return array<string, mixed> */
    public function rules(): array
    {
        return [
            'status' => ['required', 'string', Rule::in(PlayerReport::statuses())],
            'public_outcome' => ['nullable', 'string', 'max:4000'],
            'moderator_notes' => ['nullable', 'string', 'max:8000'],
            'lock_version' => ['required', 'integer', 'min:1'],
        ];
    }
}
