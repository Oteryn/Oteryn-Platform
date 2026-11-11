<?php

namespace App\Http\Requests\Admin;

use Illuminate\Foundation\Http\FormRequest;

final class AdminEditorialTranslationRequest extends FormRequest
{
    public function authorize(): bool
    {
        return true;
    }

    /** @return array<string, list<mixed>> */
    public function rules(): array
    {
        return [
            'title' => ['nullable', 'string', 'max:200'],
            'body' => ['nullable', 'string', 'max:100000'],
            'action_label' => ['nullable', 'string', 'max:80'],
            'published_at' => ['nullable', 'date_format:Y-m-d\TH:i'],
        ];
    }
}
