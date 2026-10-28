<?php

namespace App\ProductsEntitlements\Store\Http;

use App\ProductsEntitlements\Store\Delivery\StoreDeliveryResponse;
use App\ProductsEntitlements\Store\Delivery\StoreReceiptPublisher;
use Illuminate\Http\Request;
use Symfony\Component\HttpFoundation\Response;

final class StoreReceiptController
{
    public function __invoke(Request $request, StoreReceiptPublisher $exchange, StoreDeliveryResponse $response): Response
    {
        return $response->handle(fn (): string => $exchange->publish($request->getContent()));
    }
}
