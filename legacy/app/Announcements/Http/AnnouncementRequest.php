<?php

namespace App\Announcements\Http;

use App\Announcements\Links\AnnouncementActionLink;
use App\Announcements\Models\SiteAnnouncement;
use Closure;
use Illuminate\Foundation\Http\FormRequest;
use Illuminate\Validation\Rule;
use InvalidArgumentException;

final class AnnouncementRequest extends FormRequest
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
        return [
            'title' => ['required', 'string', 'max:200'],
            'body' => ['required', 'string', 'max:10000'],
            'severity' => ['required', 'string', Rule::in(SiteAnnouncement::severities())],
            'starts_at' => ['required', 'date_format:Y-m-d\TH:i'],
            'ends_at' => ['nullable', 'date_format:Y-m-d\TH:i', 'after:starts_at'],
            'publication_state' => ['required', 'string', Rule::in(SiteAnnouncement::publicationStates())],
            'action_label' => ['nullable', 'string', 'max:80', 'required_with:action_url'],
            'action_url' => [
                'nullable',
                'string',
                'max:2048',
                'required_with:action_label',
                /** @param Closure(string): void $fail */
                function (string $attribute, mixed $value, Closure $fail): void {
                    try {
                        AnnouncementActionLink::normalize(is_string($value) ? $value : null);
                    } catch (InvalidArgumentException $exception) {
                        $fail($exception->getMessage());
                    }
                },
            ],
            'lock_version' => $this->isMethod('PUT')
                ? ['required', 'integer', 'min:1']
                : ['nullable', 'integer', 'min:1'],
        ];
    }
}
