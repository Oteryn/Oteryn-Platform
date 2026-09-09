<?php

namespace App\Http\Controllers\GameAuth;

use App\GameAuth\NativeEvidence\NativeEvidenceCapacity;
use App\GameAuth\NativeEvidence\NativeEvidenceContract;
use App\GameAuth\NativeEvidence\NativeEvidenceRequestDecoder;
use App\GameAuth\NativeEvidence\NativeEvidenceSource;
use App\GameAuth\NativeEvidence\NativeEvidenceUnavailable;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use InvalidArgumentException;
use Symfony\Component\HttpFoundation\Response;

final class NativeEvidenceController
{
    public function __invoke(
        Request $request,
        NativeEvidenceRequestDecoder $decoder,
        NativeEvidenceSource $source,
        NativeEvidenceCapacity $capacity,
    ): JsonResponse|Response {
        try {
            $decoded = $decoder->decode($request->getContent());
        } catch (InvalidArgumentException) {
            return response('', 400);
        }

        try {
            $payload = $capacity->run(fn (): array => $source->observe($decoded));
        } catch (NativeEvidenceUnavailable) {
            $payload = NativeEvidenceContract::failure(
                (int) $decoded['version'],
                (string) $decoded['operation'],
                'unavailable',
            );
        }

        return response()->json(
            $payload,
            200,
            ['Cache-Control' => 'no-store, private', 'Pragma' => 'no-cache'],
            JSON_UNESCAPED_SLASHES,
        );
    }
}
