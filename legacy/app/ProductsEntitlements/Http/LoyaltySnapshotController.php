<?php

namespace App\ProductsEntitlements\Http;

use App\ProductsEntitlements\Loyalty\LoyaltySnapshotIssuer;
use App\ProductsEntitlements\Loyalty\LoyaltySnapshotRequestDecoder;
use App\ProductsEntitlements\Loyalty\LoyaltySnapshotSettings;
use App\ProductsEntitlements\Premium\PremiumSnapshotUnavailable;
use Illuminate\Database\QueryException;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use InvalidArgumentException;
use Symfony\Component\HttpFoundation\Response;

/** POST /internal/v1/products-entitlements/loyalty-snapshots/read. */
final class LoyaltySnapshotController
{
    public function __invoke(
        Request $request,
        LoyaltySnapshotRequestDecoder $decoder,
        LoyaltySnapshotIssuer $issuer,
    ): JsonResponse|Response {
        $settings = LoyaltySnapshotSettings::current();
        if ($settings === null) {
            return response('', 503);
        }

        $contentType = $request->headers->get('Content-Type');
        $contentEncoding = $request->headers->get('Content-Encoding');
        if (! is_string($contentType)
            || strtolower(trim(explode(';', $contentType, 2)[0])) !== 'application/json'
            || (is_string($contentEncoding) && strtolower(trim($contentEncoding)) !== 'identity')
            || $request->headers->has('Transfer-Encoding')) {
            return response('', 400);
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
