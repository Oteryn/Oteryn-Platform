<?php

namespace App\Http\Requests\Downloads;

use App\Downloads\DownloadCatalog;
use Illuminate\Foundation\Http\FormRequest;
use Illuminate\Validation\Rule;

final class ApproveUpdaterPolicyRequest extends FormRequest
{
    public function authorize(): bool
    {
        return true;
    }

    /** @return array<string, list<mixed>> */
    public function rules(): array
    {
        return [
            'operation_id' => ['required', 'uuid'],
            'current_release_id' => ['required', 'integer', 'min:1', 'exists:client_releases,id'],
            'minimum_supported_release_sequence' => ['required', 'integer', 'min:1'],
            'update_mode' => ['required', 'string', Rule::in(DownloadCatalog::updateModes())],
            'rollback_authorization' => [
                'required',
                'string',
                Rule::in(DownloadCatalog::rollbackAuthorizations()),
            ],
            'revoked_release_ids' => ['array', 'max:100'],
            'revoked_release_ids.*' => ['required', 'string', 'max:64', 'distinct'],
            'revoked_artifact_targets' => ['array', 'max:100'],
            'revoked_artifact_targets.*' => ['required', 'string', 'max:512', 'distinct'],
        ];
    }

    /**
     * @return array{
     *   operation_id: string,
     *   current_release_id: int,
     *   minimum_supported_release_sequence: int,
     *   update_mode: string,
     *   rollback_authorization: string,
     *   revoked_release_ids: list<string>,
     *   revoked_artifact_targets: list<string>
     * }
     */
    public function policyInput(): array
    {
        $validated = $this->validated();
        /** @var list<string> $revokedReleaseIds */
        $revokedReleaseIds = $validated['revoked_release_ids'] ?? [];
        /** @var list<string> $revokedArtifactTargets */
        $revokedArtifactTargets = $validated['revoked_artifact_targets'] ?? [];

        return [
            'operation_id' => (string) $validated['operation_id'],
            'current_release_id' => (int) $validated['current_release_id'],
            'minimum_supported_release_sequence' => (int) $validated['minimum_supported_release_sequence'],
            'update_mode' => (string) $validated['update_mode'],
            'rollback_authorization' => (string) $validated['rollback_authorization'],
            'revoked_release_ids' => array_values($revokedReleaseIds),
            'revoked_artifact_targets' => array_values($revokedArtifactTargets),
        ];
    }

    protected function prepareForValidation(): void
    {
        foreach (['operation_id', 'update_mode', 'rollback_authorization'] as $field) {
            $value = $this->input($field);
            if (is_string($value)) {
                $this->merge([$field => trim($value)]);
            }
        }

        foreach (['revoked_release_ids', 'revoked_artifact_targets'] as $field) {
            $values = $this->input($field, []);
            if (! is_array($values)) {
                continue;
            }

            $this->merge([
                $field => array_values(array_map(
                    static fn (mixed $value): mixed => is_string($value) ? trim($value) : $value,
                    $values,
                )),
            ]);
        }
    }
}
