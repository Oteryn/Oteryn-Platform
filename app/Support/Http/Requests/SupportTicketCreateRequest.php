<?php

namespace App\Support\Http\Requests;

use App\Identity\Models\Identity;
use App\Support\Models\SupportTicket;
use Illuminate\Foundation\Http\FormRequest;
use Illuminate\Validation\Rule;

final class SupportTicketCreateRequest extends FormRequest
{
    public function authorize(): bool
    {
        return $this->user() instanceof Identity;
    }

    /** @return array<string, mixed> */
    public function rules(): array
    {
        return [
            'request_key' => ['required', 'uuid', 'max:64'],
            'category' => ['required', 'string', Rule::in(SupportTicket::categories())],
            'subject' => ['required', 'string', 'max:'.config('support.tickets.subject_max_length', 160)],
            'body' => ['required', 'string', 'max:'.config('support.tickets.message_max_length', 8000)],
        ];
    }
}
