<?php

namespace App\ProductsEntitlements\Store\Http;

use Symfony\Component\HttpFoundation\Response;

final class StorePendingController
{
    public function __invoke(): Response
    {
        return response('', 503);
    }
}
