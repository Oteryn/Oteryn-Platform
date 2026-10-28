<?php

namespace App\ProductsEntitlements\Store\Http;

use Symfony\Component\HttpFoundation\Response;

final class StoreReadController
{
    public function __invoke(): Response
    {
        return response('', 503);
    }
}
