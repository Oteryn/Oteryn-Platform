<?php

namespace App\Wiki\Http\Admin\Requests;

use App\Wiki\Infrastructure\Models\WikiCategory;
use App\Wiki\Infrastructure\Models\WikiCategoryTranslation;
use Illuminate\Database\Query\Builder;
use Illuminate\Foundation\Http\FormRequest;
use Illuminate\Validation\Rule;
use Illuminate\Validation\Rules\Unique;

final class AdminWikiCategoryRequest extends FormRequest
{
    public function authorize(): bool
    {
        return true;
    }

    protected function prepareForValidation(): void
    {
        $this->merge([
            'visible' => $this->boolean('visible'),
        ]);
    }

    /** @return array<string, list<mixed>> */
    public function rules(): array
    {
        $category = $this->route('category');
        $categoryId = $category instanceof WikiCategory ? $category->id : null;

        return [
            'key' => [
                'required',
                'string',
                'max:96',
                'regex:/^[a-z0-9]+(?:[._-][a-z0-9]+)*$/',
                Rule::unique('wiki_categories', 'key')->ignore($categoryId),
            ],
            'parent_id' => [
                'nullable',
                'integer',
                'exists:wiki_categories,id',
                Rule::notIn($categoryId === null ? [] : [$categoryId]),
            ],
            'sort_order' => ['required', 'integer', 'min:0', 'max:1000000'],
            'visible' => ['required', 'boolean'],
            'translations' => ['required', 'array'],
            'translations.en' => ['required', 'array'],
            'translations.en.name' => ['required', 'string', 'max:200'],
            'translations.en.slug' => [
                'required',
                'string',
                'max:160',
                'regex:/^[a-z0-9]+(?:-[a-z0-9]+)*$/',
                $this->uniqueSlug('en', $categoryId),
            ],
            'translations.en.description' => ['nullable', 'string', 'max:10000'],
            'translations.pl' => ['nullable', 'array'],
            'translations.pl.name' => [
                'nullable',
                'string',
                'max:200',
                'required_with:translations.pl.slug,translations.pl.description',
            ],
            'translations.pl.slug' => [
                'nullable',
                'string',
                'max:160',
                'regex:/^[a-z0-9]+(?:-[a-z0-9]+)*$/',
                'required_with:translations.pl.name,translations.pl.description',
                $this->uniqueSlug('pl', $categoryId),
            ],
            'translations.pl.description' => [
                'nullable',
                'string',
                'max:10000',
                'required_with:translations.pl.name,translations.pl.slug',
            ],
            'lock_version' => $this->isMethod('PUT')
                ? ['required', 'integer', 'min:1']
                : ['nullable', 'integer', 'min:1'],
        ];
    }

    private function uniqueSlug(string $locale, ?int $categoryId): Unique
    {
        $translationId = null;

        if ($categoryId !== null) {
            $value = WikiCategoryTranslation::query()
                ->where('category_id', $categoryId)
                ->where('locale', $locale)
                ->value('id');

            if (is_int($value)) {
                $translationId = $value;
            } elseif (is_string($value) && ctype_digit($value)) {
                $translationId = (int) $value;
            }
        }

        return Rule::unique('wiki_category_translations', 'slug')
            ->where(static fn (Builder $query): Builder => $query->where('locale', $locale))
            ->ignore($translationId);
    }
}
