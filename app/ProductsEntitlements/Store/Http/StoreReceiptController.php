<?php

namespace App\ProductsEntitlements\Store\Http;

use Symfony\Component\HttpFoundation\Response;

final class StoreReceiptController
{
    public function __invoke(): Response
    {
        return response('', 503);
    }
}
