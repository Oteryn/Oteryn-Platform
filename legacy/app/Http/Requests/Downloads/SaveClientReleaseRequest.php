<?php

namespace App\Http\Requests\Downloads;

use App\Downloads\DownloadCatalog;
use App\Downloads\Models\ClientRelease;
use App\Downloads\Rules\ApprovedArtifactUrl;
use Illuminate\Database\Query\Builder;
use Illuminate\Foundation\Http\FormRequest;
use Illuminate\Validation\Rule;
use Illuminate\Validation\Validator;

final class SaveClientReleaseRequest extends FormRequest
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
        $release = $this->route('clientRelease');
        $releaseId = $release instanceof ClientRelease ? $release->id : null;
        $channel = $this->input('channel');

        return [
            'version' => [
                'required',
                'string',
                'max:64',
                'regex:/^[0-9A-Za-z][0-9A-Za-z._+\-]{0,63}$/',
                Rule::unique('client_releases', 'version')
                    ->where(static fn (Builder $query): Builder => $query->where('channel', $channel))
                    ->ignore($releaseId),
            ],
            'channel' => ['required', 'string', Rule::in(DownloadCatalog::channels())],
            'release_notes' => ['nullable', 'string', 'max:10000'],
            'artifacts' => ['required', 'array', 'min:1', 'max:12'],
            'artifacts.*' => ['required', 'array'],
            'artifacts.*.platform' => ['required', 'string', Rule::in(DownloadCatalog::platforms())],
            'artifacts.*.architecture' => ['required', 'string', Rule::in(DownloadCatalog::architectures())],
            'artifacts.*.artifact_url' => ['required', 'string', 'max:2048', new ApprovedArtifactUrl],
            'artifacts.*.filename' => [
                'required',
                'string',
                'max:255',
                'regex:/^[A-Za-z0-9][A-Za-z0-9._()+ \-]{0,254}$/',
            ],
            'artifacts.*.size_bytes' => ['required', 'integer', 'min:1', 'max:21474836480'],
            'artifacts.*.sha256' => ['required', 'string', 'regex:/^[a-f0-9]{64}$/'],
            'artifacts.*.is_enabled' => ['required', 'boolean'],
        ];
    }

    /**
     * @return list<callable(Validator): void>
     */
    public function after(): array
    {
        return [function (Validator $validator): void {
            $artifacts = $this->input('artifacts', []);

            if (! is_array($artifacts)) {
                return;
            }

            /** @var array<string, true> $variants */
            $variants = [];

            foreach ($artifacts as $index => $artifact) {
                if (! is_array($artifact)) {
                    continue;
                }

                $platform = $artifact['platform'] ?? null;
                $architecture = $artifact['architecture'] ?? null;

                if (! is_string($platform) || ! is_string($architecture)) {
                    continue;
                }

                $variant = $platform.'|'.$architecture;

                if (isset($variants[$variant])) {
                    $validator->errors()->add(
                        "artifacts.{$index}.architecture",
                        'Each operating-system and architecture variant may appear only once per release.',
                    );
                }

                $variants[$variant] = true;
            }
        }];
    }

    /**
     * @return list<array{platform: string, architecture: string, artifact_url: string, filename: string, size_bytes: int, sha256: string, is_enabled: bool}>
     */
    public function artifactInput(): array
    {
        $validated = $this->validated();

        /**
         * @var list<array{
         *     platform: string,
         *     architecture: string,
         *     artifact_url: string,
         *     filename: string,
         *     size_bytes: int|string,
         *     sha256: string,
         *     is_enabled: bool|int|string
         * }> $artifacts
         */
        $artifacts = $validated['artifacts'] ?? [];
        $result = [];

        foreach ($artifacts as $artifact) {
            $result[] = [
                'platform' => $artifact['platform'],
                'architecture' => $artifact['architecture'],
                'artifact_url' => $artifact['artifact_url'],
                'filename' => $artifact['filename'],
                'size_bytes' => (int) $artifact['size_bytes'],
                'sha256' => $artifact['sha256'],
                'is_enabled' => filter_var($artifact['is_enabled'], FILTER_VALIDATE_BOOL),
            ];
        }

        return $result;
    }

    protected function prepareForValidation(): void
    {
        $rawArtifacts = $this->input('artifacts', []);

        if (! is_array($rawArtifacts)) {
            return;
        }

        $artifacts = [];

        foreach ($rawArtifacts as $artifact) {
            if (! is_array($artifact)) {
                $artifacts[] = $artifact;

                continue;
            }

            $platform = $artifact['platform'] ?? null;
            $architecture = $artifact['architecture'] ?? null;
            $artifactUrl = $artifact['artifact_url'] ?? null;
            $filename = $artifact['filename'] ?? null;
            $sha256 = $artifact['sha256'] ?? null;
            $normalized = [
                'platform' => is_string($platform) ? trim($platform) : $platform,
                'architecture' => is_string($architecture) ? trim($architecture) : $architecture,
                'artifact_url' => is_string($artifactUrl) ? trim($artifactUrl) : $artifactUrl,
                'filename' => is_string($filename) ? trim($filename) : $filename,
                'size_bytes' => $artifact['size_bytes'] ?? null,
                'sha256' => is_string($sha256) ? strtolower(trim($sha256)) : $sha256,
                'is_enabled' => $artifact['is_enabled'] ?? false,
            ];

            $meaningful = array_filter(
                array_diff_key($normalized, ['is_enabled' => true]),
                static fn (mixed $value): bool => $value !== null && $value !== '',
            );

            if ($meaningful !== []) {
                $artifacts[] = $normalized;
            }
        }

        $version = $this->input('version');
        $channel = $this->input('channel');
        $releaseNotes = $this->input('release_notes');

        $this->merge([
            'version' => is_string($version) ? trim($version) : $version,
            'channel' => is_string($channel) ? trim($channel) : $channel,
            'release_notes' => is_string($releaseNotes) ? trim($releaseNotes) : $releaseNotes,
            'artifacts' => $artifacts,
        ]);
    }
}
