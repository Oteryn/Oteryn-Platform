<?php

namespace App\Support\Http\Requests;

use App\Identity\Models\Identity;
use App\Support\Actions\ManagePlayerReport;
use App\Support\Models\PlayerReport;
use App\Support\SupportConfiguration;
use Illuminate\Foundation\Http\FormRequest;
use Illuminate\Validation\Rule;
use Illuminate\Validation\Validator;

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
            'target_reference' => ['required', 'string', 'max:'.SupportConfiguration::positiveInteger('support.reports.target_max_length', 160)],
            'evidence_summary' => ['nullable', 'string', 'max:'.SupportConfiguration::positiveInteger('support.reports.evidence_max_length', 4000)],
        ];
    }

    /** @return list<callable(Validator): void> */
    public function after(): array
    {
        return [
            function (Validator $validator): void {
                $type = $this->input('report_type');
                $category = $this->input('category');
                $categories = ManagePlayerReport::categories();

                if (! is_string($type) || ! is_string($category)) {
                    return;
                }

                if (! isset($categories[$type]) || ! in_array($category, $categories[$type], true)) {
                    $validator->errors()->add('category', 'The selected category is not valid for this report type.');
                }
            },
        ];
    }
}
