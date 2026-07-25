<?php

namespace App\EditorialMedia\Application;

use App\EditorialMedia\Domain\EditorialMediaConsumer;
use App\EditorialMedia\Infrastructure\Models\EditorialMedia;
use App\EditorialMedia\Infrastructure\Models\EditorialMediaReference;
use Illuminate\Support\Facades\DB;
use Illuminate\Validation\ValidationException;

final class EditorialMediaReferenceManager
{
    public function attach(
        EditorialMedia $media,
        EditorialMediaConsumer $consumer,
        string $consumerId,
        string $usage,
    ): EditorialMediaReference {
        $this->assertReferencePart($consumerId, 'consumer_id', 64, '/^[A-Za-z0-9._:-]+$/');
        $this->assertReferencePart($usage, 'usage', 64, '/^[a-z][a-z0-9._-]*$/');

        return DB::transaction(function () use ($media, $consumer, $consumerId, $usage): EditorialMediaReference {
            $lockedMedia = EditorialMedia::query()
                ->lockForUpdate()
                ->findOrFail($media->id);

            return EditorialMediaReference::query()->updateOrCreate(
                [
                    'consumer' => $consumer->value,
                    'consumer_id' => $consumerId,
                    'usage' => $usage,
                ],
                ['media_id' => $lockedMedia->id],
            );
        }, 3);
    }

    public function release(
        EditorialMediaConsumer $consumer,
        string $consumerId,
        string $usage,
    ): void {
        $this->assertReferencePart($consumerId, 'consumer_id', 64, '/^[A-Za-z0-9._:-]+$/');
        $this->assertReferencePart($usage, 'usage', 64, '/^[a-z][a-z0-9._-]*$/');

        EditorialMediaReference::query()
            ->where('consumer', $consumer->value)
            ->where('consumer_id', $consumerId)
            ->where('usage', $usage)
            ->delete();
    }

    private function assertReferencePart(string $value, string $field, int $maxLength, string $pattern): void
    {
        if ($value === '' || strlen($value) > $maxLength || preg_match($pattern, $value) !== 1) {
            throw ValidationException::withMessages([
                $field => 'The media reference identifier is invalid.',
            ]);
        }
    }
}
