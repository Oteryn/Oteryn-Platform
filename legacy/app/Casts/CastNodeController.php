<?php

namespace App\Casts;

use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use Illuminate\Support\Str;

final class CastNodeController
{
    public function __invoke(Request $request, CastControls $controls): JsonResponse
    {
        try {
            if (strlen($request->getContent()) > 4096 || ! $request->isJson()) {
                throw new CastUnavailable;
            }
            $wire = json_decode($request->getContent(), true, 8, JSON_THROW_ON_ERROR);
            if (! is_array($wire)) {
                throw new CastUnavailable;
            }
            $node = $request->attributes->get('cast_node');
            if (! is_string($node)) {
                throw new CastUnavailable;
            }
            $operation = $wire['operation'] ?? null;
            $result = match ($operation) {
                'consume' => $controls->consumeTicket($node, $this->text($wire, 'ticket', 69)),
                'renew' => $controls->consume($node, $this->uuid($wire, 'reservation'), true),
                'release' => $controls->release($node, $this->uuid($wire, 'reservation')),
                'stop' => $controls->stop($node, $this->text($wire, 'cast', 32)),
                'announce' => $controls->announce($node, $this->uuid($wire, 'owner'),
                    $this->text($wire, 'cast', 32), $this->integer($wire, 'epoch'),
                    $this->integer($wire, 'accessRevision'), $this->integer($wire, 'delaySeconds'),
                    $this->integer($wire, 'viewerCap'), $this->boolean($wire, 'passwordRequired')),
                default => throw new CastUnavailable,
            };

            return response()->json($result instanceof CastReservation ? [
                'purpose' => 'oteryn-cast-observer-v1', 'node' => $result->nodeId,
                'account' => $result->accountId, 'cast' => $result->castId,
                'epoch' => $result->epoch, 'accessRevision' => $result->accessRevision,
                'reservation' => $result->reservationId, 'expiresAt' => $result->expiresAt, 'chat' => $result->chatSelected,
            ] : ['applied' => true]);
        } catch (CastUnavailable|\JsonException) {
            return response()->json(['error' => 'Cast unavailable'], 409);
        }
    }

    /** @param array<array-key, mixed> $wire */
    private function text(array $wire, string $key, int $max): string
    {
        $value = $wire[$key] ?? null;
        if (! is_string($value) || $value === '' || strlen($value) > $max) {
            throw new CastUnavailable;
        }

        return $value;
    }

    /** @param array<array-key, mixed> $wire */
    private function uuid(array $wire, string $key): string
    {
        $value = $this->text($wire, $key, 36);
        if (! Str::isUuid($value)) {
            throw new CastUnavailable;
        }

        return $value;
    }

    /** @param array<array-key, mixed> $wire */
    private function integer(array $wire, string $key): int
    {
        $value = $wire[$key] ?? null;
        if (! is_int($value)) {
            throw new CastUnavailable;
        }

        return $value;
    }

    /** @param array<array-key, mixed> $wire */
    private function boolean(array $wire, string $key): bool
    {
        $value = $wire[$key] ?? null;
        if (! is_bool($value)) {
            throw new CastUnavailable;
        }

        return $value;
    }
}
