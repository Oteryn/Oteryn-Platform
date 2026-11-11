<?php

namespace App\Http\Requests\Admin;

use App\Cms\Editorial\EditorialPageKey;
use Illuminate\Foundation\Http\FormRequest;

final class AdminSupportContentRequest extends FormRequest
{
    public function authorize(): bool
    {
        return true;
    }

    /**
     * @return array<string, list<mixed>>
     */
    public function rules(): array
    {
        $key = EditorialPageKey::tryFrom((string) $this->route('editorialPageKey'));
        $isLegal = $key?->isLegal() ?? false;
        $isPublishing = $this->filled('published_at');

        /** @var array<string, list<mixed>> $rules */
        $rules = [
            'title' => ['required', 'string', 'max:200'],
            'body' => ['required', 'string', 'max:100000'],
            'published_at' => ['nullable', 'date'],
        ];

        if ($isLegal) {
            $rules['legal_version'] = [
                $isPublishing ? 'required' : 'nullable',
                'string',
                'max:40',
                'regex:/^[A-Za-z0-9][A-Za-z0-9._-]{0,39}$/',
            ];
            $rules['legal_effective_date'] = [
                $isPublishing ? 'required' : 'nullable',
                'date',
            ];
        } else {
            $rules['legal_version'] = ['prohibited'];
            $rules['legal_effective_date'] = ['prohibited'];
        }

        return $rules;
    }
}
