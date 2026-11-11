<?php

namespace App\Downloads\Factories;

use App\Downloads\DownloadCatalog;
use App\Downloads\Models\ClientReleaseArtifact;
use Illuminate\Database\Eloquent\Factories\Factory;

/**
 * @extends Factory<ClientReleaseArtifact>
 */
final class ClientReleaseArtifactFactory extends Factory
{
    /**
     * @var class-string<ClientReleaseArtifact>
     */
    protected $model = ClientReleaseArtifact::class;

    /**
     * @return array<string, mixed>
     */
    public function definition(): array
    {
        return [
            'client_release_id' => ClientReleaseFactory::new(),
            'platform' => DownloadCatalog::PLATFORM_WINDOWS,
            'architecture' => DownloadCatalog::ARCHITECTURE_X86_64,
            'artifact_url' => 'https://downloads.example.test/releases/oteryn-client.zip',
            'filename' => 'oteryn-client.zip',
            'size_bytes' => 104857600,
            'sha256' => str_repeat('a', 64),
            'is_enabled' => true,
        ];
    }
}
