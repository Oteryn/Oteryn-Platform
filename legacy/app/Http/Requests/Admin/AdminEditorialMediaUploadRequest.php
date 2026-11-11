<?php

namespace App\Http\Requests\Admin;

use Illuminate\Foundation\Http\FormRequest;
use RuntimeException;

final class AdminEditorialMediaUploadRequest extends FormRequest
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
        $maxBytes = config('editorial_media.max_bytes');

        if (! is_int($maxBytes) || $maxBytes < 1) {
            throw new RuntimeException('Editorial image upload configuration is invalid.');
        }

        return [
            'image' => ['bail', 'required', 'file', 'max:'.(int) ceil($maxBytes / 1024)],
            'alt_text' => ['bail', 'required', 'string', 'max:500'],
        ];
    }
}
