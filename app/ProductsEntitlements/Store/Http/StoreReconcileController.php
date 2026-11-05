<?php

namespace App\ProductsEntitlements\Store\Http;

use App\ProductsEntitlements\Store\Delivery\StoreDeliveryFeed;
use App\ProductsEntitlements\Store\Delivery\StoreDeliveryResponse;
use Illuminate\Http\Request;
use Symfony\Component\HttpFoundation\Response;

final class StoreReconcileController
{
    public function __invoke(Request $request, StoreDeliveryFeed $exchange, StoreDeliveryResponse $response): Response
    {
        return $response->handle(fn (): string => $exchange->reconcile($request->getContent()));
    }
}
