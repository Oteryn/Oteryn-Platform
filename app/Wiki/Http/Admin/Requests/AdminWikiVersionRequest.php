<?php

namespace App\Wiki\Http\Admin\Requests;

use Illuminate\Foundation\Http\FormRequest;

class AdminWikiVersionRequest extends FormRequest
{
    public function authorize(): bool
    {
        return true;
    }

    /** @return array<string, list<mixed>> */
    public function rules(): array
    {
        return [
            'lock_version' => ['required', 'integer', 'min:1'],
        ];
    }
}
