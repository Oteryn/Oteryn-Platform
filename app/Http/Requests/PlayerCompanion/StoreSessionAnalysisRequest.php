<?php

namespace App\Http\Requests\PlayerCompanion;

use Illuminate\Foundation\Http\FormRequest;
use Illuminate\Validation\Validator;

final class StoreSessionAnalysisRequest extends FormRequest
{
    private const MAX_LOG_BYTES = 65_535;

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

    public function withValidator(Validator $validator): void
    {
        $validator->after(function (Validator $validator): void {
            $log = $this->input('session_log');
            if (is_string($log) && strlen($log) > self::MAX_LOG_BYTES) {
                $validator->errors()->add('session_log', __('player_companion.payload_too_large'));
            }
        });
    }

    /** @return array<string, string> */
    public function messages(): array
    {
        return [
            'session_log.max' => __('player_companion.payload_too_large'),
        ];
    }

    protected function failedValidation(Validator $validator): void
    {
        // Raw pasted session text is private input. Never flash it into the session on validation failure.
        $this->request->remove('session_log');

        parent::failedValidation($validator);
    }
}
