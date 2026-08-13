<?php

namespace App\Http\Requests\PlayerCompanion;

use Illuminate\Foundation\Http\FormRequest;

final class StoreSessionAnalysisRequest extends FormRequest
{
    public function authorize(): bool
    {
        return $this->user() !== null;
    }

    /** @return array<string, list<string>> */
    public function rules(): array
    {
        return [
            'label' => ['nullable', 'string', 'max:80'],
            'session_log' => ['required', 'string', 'max:65535'],
        ];
    }
}
