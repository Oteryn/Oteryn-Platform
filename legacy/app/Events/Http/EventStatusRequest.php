<?php

namespace App\Events\Http;

use App\Events\Models\Event;
use Illuminate\Foundation\Http\FormRequest;
use Illuminate\Validation\Rule;

final class EventStatusRequest extends FormRequest
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
            'status' => ['required', 'string', Rule::in(Event::statuses())],
            'lock_version' => ['required', 'integer', 'min:1'],
        ];
    }
}
