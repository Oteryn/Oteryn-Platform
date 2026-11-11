<?php

namespace App\Http\Requests\Admin;

use App\Cms\Editorial\EditorialPageKey;
use App\Cms\Models\ManagedPage;
use Illuminate\Foundation\Http\FormRequest;
use Illuminate\Validation\Rule;

final class AdminManagedPageRequest extends FormRequest
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
        $page = $this->route('managedPage');
        $pageId = $page instanceof ManagedPage ? $page->id : null;

        return [
            'slug' => [
                'required',
                'string',
                'max:160',
                'regex:/^[a-z0-9]+(?:-[a-z0-9]+)*$/',
                Rule::notIn(EditorialPageKey::managedPageSlugs()),
                Rule::unique('managed_pages', 'slug')->ignore($pageId),
            ],
            'title' => ['required', 'string', 'max:200'],
            'body' => ['required', 'string', 'max:100000'],
            'published_at' => ['nullable', 'date'],
        ];
    }
}
