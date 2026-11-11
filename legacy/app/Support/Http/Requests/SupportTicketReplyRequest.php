<?php

namespace App\Support\Http\Requests;

use App\Identity\Models\Identity;
use App\Support\SupportConfiguration;
use Illuminate\Foundation\Http\FormRequest;

final class SupportTicketReplyRequest extends FormRequest
{
    public function authorize(): bool
    {
        return $this->user() instanceof Identity;
    }

    /** @return array<string, mixed> */
    public function rules(): array
    {
        return [
            'body' => ['required', 'string', 'max:'.SupportConfiguration::positiveInteger('support.tickets.message_max_length', 8000)],
            'internal' => ['sometimes', 'boolean'],
            'lock_version' => ['required', 'integer', 'min:1'],
        ];
    }
}
