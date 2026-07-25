<?php

namespace App\Downloads\Factories;

use App\Downloads\DownloadCatalog;
use App\Downloads\Models\ClientRelease;
use Illuminate\Database\Eloquent\Factories\Factory;

/**
 * @extends Factory<ClientRelease>
 */
final class ClientReleaseFactory extends Factory
{
    /**
     * @var class-string<ClientRelease>
     */
    protected $model = ClientRelease::class;

    /**
     * @return array<string, mixed>
     */
    public function definition(): array
    {
        return [
            'version' => '1.0.'.random_int(1, 999999),
            'channel' => DownloadCatalog::CHANNEL_STABLE,
            'release_notes' => 'Test client release.',
            'published_at' => null,
            'is_current' => false,
        ];
    }

    public function published(bool $current = true): self
    {
        return $this->state([
            'published_at' => now()->subMinute(),
            'is_current' => $current,
        ]);
    }
}
