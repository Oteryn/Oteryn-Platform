<?php

namespace App\Http\Requests\Downloads;

use Illuminate\Foundation\Http\FormRequest;

final class PublishClientReleaseRequest extends FormRequest
{
    public function authorize(): bool
    {
        return true;
    }

    /**
     * @return array<string, list<string>>
     */
    public function rules(): array
    {
        return [
            'make_current' => ['required', 'boolean'],
        ];
    }

    public function makeCurrent(): bool
    {
        return $this->boolean('make_current');
    }
}
