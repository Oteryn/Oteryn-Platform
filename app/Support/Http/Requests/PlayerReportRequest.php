<?php

namespace App\Support\Http\Requests;

use App\Identity\Models\Identity;
use App\Support\Actions\ManagePlayerReport;
use App\Support\Models\PlayerReport;
use Illuminate\Foundation\Http\FormRequest;
use Illuminate\Validation\Rule;

final class PlayerReportRequest extends FormRequest
{
    public function authorize(): bool
    {
        return $this->user() instanceof Identity;
    }

    /** @return array<string, mixed> */
    public function rules(): array
    {
        $categories = array_values(array_unique(array_merge(...array_values(ManagePlayerReport::categories()))));

        return [
            'request_key' => ['required', 'uuid', 'max:64'],
            'report_type' => ['required', 'string', Rule::in(PlayerReport::types())],
            'category' => ['required', 'string', Rule::in($categories)],
            'target_reference' => ['required', 'string', 'max:'.config('support.reports.target_max_length', 160)],
            'evidence_summary' => ['nullable', 'string', 'max:'.config('support.reports.evidence_max_length', 4000)],
        ];
    }
}
