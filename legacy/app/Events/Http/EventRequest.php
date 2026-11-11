<?php

namespace App\Events\Http;

use App\Events\Models\Event;
use App\Events\Models\EventTranslation;
use Illuminate\Database\Query\Builder;
use Illuminate\Foundation\Http\FormRequest;
use Illuminate\Validation\Rule;
use Illuminate\Validation\Rules\Unique;

final class EventRequest extends FormRequest
{
    public function authorize(): bool
    {
        return true;
    }

    protected function prepareForValidation(): void
    {
        $this->merge([
            'featured' => $this->boolean('featured'),
        ]);
    }

    /**
     * @return array<string, list<mixed>>
     */
    public function rules(): array
    {
        $event = $this->route('event');
        $eventId = $event instanceof Event ? $event->id : null;

        return [
            'starts_at' => ['required', 'date_format:Y-m-d\TH:i'],
            'ends_at' => ['required', 'date_format:Y-m-d\TH:i', 'after:starts_at'],
            'featured' => ['required', 'boolean'],
            'news_post_id' => ['nullable', 'integer', 'exists:news_posts,id'],
            'translations' => ['required', 'array'],
            'translations.en' => ['required', 'array'],
            'translations.en.title' => ['required', 'string', 'max:200'],
            'translations.en.slug' => [
                'required',
                'string',
                'max:160',
                'regex:/^[a-z0-9]+(?:-[a-z0-9]+)*$/',
                $this->uniqueSlug('en', $eventId),
            ],
            'translations.en.summary' => ['required', 'string', 'max:500'],
            'translations.en.body' => ['required', 'string', 'max:100000'],
            'translations.pl' => ['nullable', 'array'],
            'translations.pl.title' => ['nullable', 'string', 'max:200', 'required_with:translations.pl.slug,translations.pl.summary,translations.pl.body'],
            'translations.pl.slug' => [
                'nullable',
                'string',
                'max:160',
                'regex:/^[a-z0-9]+(?:-[a-z0-9]+)*$/',
                'required_with:translations.pl.title,translations.pl.summary,translations.pl.body',
                $this->uniqueSlug('pl', $eventId),
            ],
            'translations.pl.summary' => ['nullable', 'string', 'max:500', 'required_with:translations.pl.title,translations.pl.slug,translations.pl.body'],
            'translations.pl.body' => ['nullable', 'string', 'max:100000', 'required_with:translations.pl.title,translations.pl.slug,translations.pl.summary'],
            'lock_version' => $this->isMethod('PUT')
                ? ['required', 'integer', 'min:1']
                : ['nullable', 'integer', 'min:1'],
        ];
    }

    private function uniqueSlug(string $locale, ?int $eventId): Unique
    {
        $translationId = null;

        if ($eventId !== null) {
            $rawTranslationId = EventTranslation::query()
                ->where('event_id', $eventId)
                ->where('locale', $locale)
                ->value('id');

            if (is_int($rawTranslationId)) {
                $translationId = $rawTranslationId;
            } elseif (is_string($rawTranslationId) && ctype_digit($rawTranslationId)) {
                $translationId = (int) $rawTranslationId;
            }
        }

        return Rule::unique('event_translations', 'slug')
            ->where(static fn (Builder $query): Builder => $query->where('locale', $locale))
            ->ignore($translationId);
    }
}
