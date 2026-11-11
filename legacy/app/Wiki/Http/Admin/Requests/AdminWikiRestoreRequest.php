<?php

namespace App\Wiki\Http\Admin\Requests;

final class AdminWikiRestoreRequest extends AdminWikiVersionRequest
{
    /** @return array<string, list<mixed>> */
    public function rules(): array
    {
        return [
            ...parent::rules(),
            'change_note' => ['nullable', 'string', 'max:500'],
        ];
    }
}
