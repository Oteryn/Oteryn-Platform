<?php

namespace App\Support\Http\Requests;

use App\Identity\Models\Identity;
use App\Support\Models\SupportTicket;
use Illuminate\Foundation\Http\FormRequest;
use Illuminate\Validation\Rule;

final class SupportTicketStatusRequest extends FormRequest
{
    public function authorize(): bool
    {
        return $this->user() instanceof Identity;
    }

    /** @return array<string, mixed> */
    public function rules(): array
    {
        return [
            'status' => ['required', 'string', Rule::in(SupportTicket::statuses())],
            'lock_version' => ['required', 'integer', 'min:1'],
        ];
    }
}
