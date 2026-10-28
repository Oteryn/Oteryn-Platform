<?php

namespace App\ProductsEntitlements\Store\Http;

use Symfony\Component\HttpFoundation\Response;

final class StoreHistoryController
{
    public function __invoke(): Response
    {
        return response('', 503);
    }
}
