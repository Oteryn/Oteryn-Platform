<?php

namespace App\Wiki\Http\Admin\Requests;

use App\Wiki\Infrastructure\Models\WikiArticle;
use App\Wiki\Infrastructure\Models\WikiArticleTranslation;
use Illuminate\Database\Query\Builder;
use Illuminate\Foundation\Http\FormRequest;
use Illuminate\Validation\Rule;
use Illuminate\Validation\Rules\Unique;

final class AdminWikiArticleRequest extends FormRequest
{
    public function authorize(): bool
    {
        return true;
    }

    protected function prepareForValidation(): void
    {
        $this->merge([
            'is_featured' => $this->boolean('is_featured'),
        ]);
    }

    /** @return array<string, list<mixed>> */
    public function rules(): array
    {
        $article = $this->route('article');
        $articleId = $article instanceof WikiArticle ? $article->id : null;

        return [
            'content_type' => [
                'required',
                'string',
                'max:64',
                'regex:/^[a-z0-9]+(?:[._-][a-z0-9]+)*$/',
            ],
            'is_featured' => ['required', 'boolean'],
            'sort_order' => ['required', 'integer', 'min:0', 'max:1000000'],
            'category_ids' => ['nullable', 'array', 'max:50'],
            'category_ids.*' => ['integer', 'distinct', 'exists:wiki_categories,id'],
            'translations' => ['required', 'array'],
            'translations.en' => ['required', 'array'],
            'translations.en.title' => ['required', 'string', 'max:200'],
            'translations.en.slug' => [
                'required',
                'string',
                'max:160',
                'regex:/^[a-z0-9]+(?:-[a-z0-9]+)*$/',
                $this->uniqueSlug('en', $articleId),
            ],
            'translations.en.summary' => ['nullable', 'string', 'max:1000'],
            'translations.en.source_markdown' => ['nullable', 'string', 'max:100000'],
            'translations.pl' => ['nullable', 'array'],
            'translations.pl.title' => [
                'nullable',
                'string',
                'max:200',
                'required_with:translations.pl.slug,translations.pl.summary,translations.pl.source_markdown',
            ],
            'translations.pl.slug' => [
                'nullable',
                'string',
                'max:160',
                'regex:/^[a-z0-9]+(?:-[a-z0-9]+)*$/',
                'required_with:translations.pl.title,translations.pl.summary,translations.pl.source_markdown',
                $this->uniqueSlug('pl', $articleId),
            ],
            'translations.pl.summary' => [
                'nullable',
                'string',
                'max:1000',
                'required_with:translations.pl.title,translations.pl.slug,translations.pl.source_markdown',
            ],
            'translations.pl.source_markdown' => [
                'nullable',
                'string',
                'max:100000',
                'required_with:translations.pl.title,translations.pl.slug,translations.pl.summary',
            ],
            'change_note' => ['nullable', 'string', 'max:500'],
            'lock_version' => $this->isMethod('PUT')
                ? ['required', 'integer', 'min:1']
                : ['nullable', 'integer', 'min:1'],
        ];
    }

    private function uniqueSlug(string $locale, ?int $articleId): Unique
    {
        $translationId = null;

        if ($articleId !== null) {
            $value = WikiArticleTranslation::query()
                ->where('article_id', $articleId)
                ->where('locale', $locale)
                ->value('id');

            if (is_int($value)) {
                $translationId = $value;
            } elseif (is_string($value) && ctype_digit($value)) {
                $translationId = (int) $value;
            }
        }

        return Rule::unique('wiki_article_translations', 'slug')
            ->where(static fn (Builder $query): Builder => $query->where('locale', $locale))
            ->ignore($translationId);
    }
}
