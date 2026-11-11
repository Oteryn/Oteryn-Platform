<?php

namespace App\ProductsEntitlements\Store\Http;

use App\ProductsEntitlements\Store\Purchase\PurchaseStoreOffer;
use App\ProductsEntitlements\Store\Purchase\StoreAccountLimiter;
use App\ProductsEntitlements\Store\Purchase\StorePurchaseInput;
use App\ProductsEntitlements\Store\StoreSettings;
use Illuminate\Database\QueryException;
use Illuminate\Http\Request;
use InvalidArgumentException;
use RuntimeException;
use Symfony\Component\HttpFoundation\Response;

final class StorePurchaseController
{
    public function __invoke(Request $request, StorePurchaseInput $decoder, PurchaseStoreOffer $purchase, StoreAccountLimiter $limiter): Response
    {
        try {
            $input = $decoder->decode($request->getContent());
        } catch (InvalidArgumentException) {
            return response('', 400);
        }
        $settings = StoreSettings::current('purchase');
        if ($settings === null) {
            return response('', 503);
        }
        $limited = $limiter->refuse($input['account_id'], $settings->purchasesPerAccountPerMinute, 'purchase');
        if ($limited !== null) {
            return $limited;
        }
        try {
            return $purchase->execute($input, $request->getContent(), $settings);
        } catch (QueryException|RuntimeException) {
            return response('', 503);
        }
    }
}
