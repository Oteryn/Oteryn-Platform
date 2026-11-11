<?php

namespace App\Http\Requests\Admin;

use App\Admin\AdminRoleManager;
use Illuminate\Foundation\Http\FormRequest;
use Illuminate\Validation\Rule;

final class AdminRoleAssignmentRequest extends FormRequest
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
            'role' => ['required', 'string', Rule::in(AdminRoleManager::roles())],
        ];
    }
}
