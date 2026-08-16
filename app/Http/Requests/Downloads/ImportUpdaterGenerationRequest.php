<?php

namespace App\Http\Requests\Downloads;

use App\Downloads\DownloadCatalog;
use Illuminate\Foundation\Http\FormRequest;
use Illuminate\Validation\Rule;
use Illuminate\Validation\Validator;
use JsonException;

final class ImportUpdaterGenerationRequest extends FormRequest
{
    private const TOP_LEVEL_KEYS = [
        'generation_id',
        'channel',
        'policy_revision',
        'root_version',
        'targets_version',
        'snapshot_version',
        'timestamp_version',
        'metadata_expires_at',
        'metadata_set_sha256',
        'policy_target_path',
        'policy_target_sha256',
        'policy_target_length',
        'targets',
    ];

    private const TARGET_KEYS = ['platform', 'architecture', 'target_path', 'length', 'sha256'];

    public function authorize(): bool
    {
        return true;
    }

    /** @return array<string, list<mixed>> */
    public function rules(): array
    {
        return [
            'public_metadata_json' => ['required', 'string', 'max:200000'],
            'generation_payload' => ['required', 'array'],
            'generation_payload.generation_id' => [
                'required',
                'string',
                'max:128',
                'regex:/^[0-9A-Za-z][0-9A-Za-z._:-]{0,127}$/',
            ],
            'generation_payload.channel' => ['required', 'string', Rule::in(DownloadCatalog::channels())],
            'generation_payload.policy_revision' => ['required', 'integer', 'min:1'],
            'generation_payload.root_version' => ['required', 'integer', 'min:1'],
            'generation_payload.targets_version' => ['required', 'integer', 'min:1'],
            'generation_payload.snapshot_version' => ['required', 'integer', 'min:1'],
            'generation_payload.timestamp_version' => ['required', 'integer', 'min:1'],
            'generation_payload.metadata_expires_at' => ['required', 'date'],
            'generation_payload.metadata_set_sha256' => ['required', 'string', 'regex:/^[a-f0-9]{64}$/'],
            'generation_payload.policy_target_path' => ['required', 'string', 'max:255'],
            'generation_payload.policy_target_sha256' => ['required', 'string', 'regex:/^[a-f0-9]{64}$/'],
            'generation_payload.policy_target_length' => ['required', 'integer', 'min:1'],
            'generation_payload.targets' => ['required', 'array', 'min:1', 'max:12'],
            'generation_payload.targets.*' => ['required', 'array'],
            'generation_payload.targets.*.platform' => [
                'required',
                'string',
                Rule::in(DownloadCatalog::platforms()),
            ],
            'generation_payload.targets.*.architecture' => [
                'required',
                'string',
                Rule::in(DownloadCatalog::architectures()),
            ],
            'generation_payload.targets.*.target_path' => ['required', 'string', 'max:512'],
            'generation_payload.targets.*.length' => ['required', 'integer', 'min:1'],
            'generation_payload.targets.*.sha256' => ['required', 'string', 'regex:/^[a-f0-9]{64}$/'],
        ];
    }

    /** @return list<callable(Validator): void> */
    public function after(): array
    {
        return [function (Validator $validator): void {
            $payload = $this->input('generation_payload');
            if (! is_array($payload)) {
                return;
            }

            $unknownTopLevel = array_diff(array_keys($payload), self::TOP_LEVEL_KEYS);
            if ($unknownTopLevel !== []) {
                $validator->errors()->add(
                    'public_metadata_json',
                    'The public signed-generation projection contains unsupported fields. Private keys, secrets and unmodelled metadata are rejected.',
                );
            }

            $routeChannel = $this->route('channel');
            $payloadChannel = $payload['channel'] ?? null;
            if (is_string($routeChannel)
                && is_string($payloadChannel)
                && $routeChannel !== $payloadChannel) {
                $validator->errors()->add(
                    'generation_payload.channel',
                    'The signed-generation channel must exactly match the administration route channel.',
                );
            }

            $targets = $payload['targets'] ?? null;
            if (! is_array($targets)) {
                return;
            }

            $variants = [];
            $paths = [];
            foreach ($targets as $index => $target) {
                if (! is_array($target)) {
                    continue;
                }

                if (array_diff(array_keys($target), self::TARGET_KEYS) !== []) {
                    $validator->errors()->add(
                        "generation_payload.targets.{$index}",
                        'Signed target projections may contain only the exact public target fields.',
                    );
                }

                $platform = $target['platform'] ?? null;
                $architecture = $target['architecture'] ?? null;
                $path = $target['target_path'] ?? null;
                if (! is_string($platform) || ! is_string($architecture) || ! is_string($path)) {
                    continue;
                }

                $variant = $platform.'|'.$architecture;
                if (isset($variants[$variant])) {
                    $validator->errors()->add(
                        "generation_payload.targets.{$index}.architecture",
                        'Each platform/architecture target must be unique.',
                    );
                }
                if (isset($paths[$path])) {
                    $validator->errors()->add(
                        "generation_payload.targets.{$index}.target_path",
                        'Each signed target path must be unique.',
                    );
                }
                $variants[$variant] = true;
                $paths[$path] = true;
            }
        }];
    }

    /**
     * @return array{
     *   generation_id: string,
     *   channel: string,
     *   policy_revision: int,
     *   root_version: int,
     *   targets_version: int,
     *   snapshot_version: int,
     *   timestamp_version: int,
     *   metadata_expires_at: string,
     *   metadata_set_sha256: string,
     *   policy_target_path: string,
     *   policy_target_sha256: string,
     *   policy_target_length: int,
     *   targets: list<array{platform: string, architecture: string, target_path: string, length: int, sha256: string}>
     * }
     */
    public function generationPayload(): array
    {
        $validated = $this->validated();
        /** @var array<string, mixed> $payload */
        $payload = $validated['generation_payload'];
        /** @var list<array<string, mixed>> $targets */
        $targets = $payload['targets'];

        return [
            'generation_id' => (string) $payload['generation_id'],
            'channel' => (string) $payload['channel'],
            'policy_revision' => (int) $payload['policy_revision'],
            'root_version' => (int) $payload['root_version'],
            'targets_version' => (int) $payload['targets_version'],
            'snapshot_version' => (int) $payload['snapshot_version'],
            'timestamp_version' => (int) $payload['timestamp_version'],
            'metadata_expires_at' => (string) $payload['metadata_expires_at'],
            'metadata_set_sha256' => strtolower((string) $payload['metadata_set_sha256']),
            'policy_target_path' => (string) $payload['policy_target_path'],
            'policy_target_sha256' => strtolower((string) $payload['policy_target_sha256']),
            'policy_target_length' => (int) $payload['policy_target_length'],
            'targets' => array_map(
                static fn (array $target): array => [
                    'platform' => (string) $target['platform'],
                    'architecture' => (string) $target['architecture'],
                    'target_path' => (string) $target['target_path'],
                    'length' => (int) $target['length'],
                    'sha256' => strtolower((string) $target['sha256']),
                ],
                $targets,
            ),
        ];
    }

    protected function prepareForValidation(): void
    {
        $raw = $this->input('public_metadata_json');
        if (! is_string($raw)) {
            return;
        }

        try {
            $payload = json_decode($raw, true, 64, JSON_THROW_ON_ERROR);
        } catch (JsonException) {
            $payload = null;
        }

        $this->merge(['generation_payload' => is_array($payload) ? $payload : null]);
    }
}