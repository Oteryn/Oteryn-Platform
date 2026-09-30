<?php

namespace App\ProductsEntitlements\Http;

use App\ProductsEntitlements\Premium\PremiumSnapshotIssuer;
use App\ProductsEntitlements\Premium\PremiumSnapshotRequestDecoder;
use App\ProductsEntitlements\Premium\PremiumSnapshotSettings;
use App\ProductsEntitlements\Premium\PremiumSnapshotUnavailable;
use Illuminate\Database\QueryException;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use InvalidArgumentException;
use Symfony\Component\HttpFoundation\Response;

/** POST /internal/v1/products-entitlements/premium-snapshots/read (OTERYN_V2_PREMIUM_TIME_SNAPSHOT_CONTRACT.md). */
final class PremiumSnapshotController
{
    public function __invoke(
        Request $request,
        PremiumSnapshotRequestDecoder $decoder,
        PremiumSnapshotIssuer $issuer,
    ): JsonResponse|Response {
        $settings = PremiumSnapshotSettings::current();
        if ($settings === null) {
            return response('', 503);
        }

        try {
            $decoded = $decoder->decode($request->getContent());
        } catch (InvalidArgumentException) {
            return response('', 400);
        }

        try {
            $snapshot = $issuer->issue($decoded['account_id'], $decoded['nonce'], $settings->producerRevision);
        } catch (PremiumSnapshotUnavailable|QueryException) {
            return response('', 503);
        }
        if ($snapshot === null) {
            return response('', 404);
        }

        return response()->json($snapshot, 200, [], JSON_UNESCAPED_SLASHES);
    }
}
