<?php

namespace App\ProductsEntitlements\Store\Http;

use App\ProductsEntitlements\Store\Purchase\ReadStoreAccount;
use App\ProductsEntitlements\Store\Purchase\StoreAccountLimiter;
use App\ProductsEntitlements\Store\Purchase\StoreReadInput;
use App\ProductsEntitlements\Store\StoreSettings;
use Illuminate\Http\Request;
use InvalidArgumentException;
use RuntimeException;
use Symfony\Component\HttpFoundation\Response;

final class StoreHistoryController
{
    public function __invoke(Request $request, StoreReadInput $decoder, ReadStoreAccount $reader, StoreAccountLimiter $limiter): Response
    {
        try {
            $input = $decoder->decode($request->getContent(), true);
        } catch (InvalidArgumentException) {
            return response('', 400);
        }
        $settings = StoreSettings::current('purchase');
        if ($settings === null) {
            return response('', 503);
        }
        $limited = $limiter->refuse($input['account_id'], $settings->readsPerAccountPerMinute, 'read');
        if ($limited !== null) {
            return $limited;
        }
        try {
            return $reader->read($input, true);
        } catch (RuntimeException) {
            return response('', 503);
        }
    }
}
