<?php

namespace App\Http\Requests\Admin;

use App\Cms\Models\NewsPost;
use Illuminate\Foundation\Http\FormRequest;
use Illuminate\Validation\Rule;

final class AdminNewsPostRequest extends FormRequest
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
        $post = $this->route('newsPost');
        $postId = $post instanceof NewsPost ? $post->id : null;

        return [
            'slug' => [
                'required',
                'string',
                'max:160',
                'regex:/^[a-z0-9]+(?:-[a-z0-9]+)*$/',
                Rule::unique('news_posts', 'slug')->ignore($postId),
            ],
            'title' => ['required', 'string', 'max:200'],
            'body' => ['required', 'string', 'max:100000'],
            'published_at' => ['nullable', 'date'],
        ];
    }
}
