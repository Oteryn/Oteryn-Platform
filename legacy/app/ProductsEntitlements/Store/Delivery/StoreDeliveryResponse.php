<?php

namespace App\ProductsEntitlements\Store\Delivery;

use App\CharacterAuthority\CharacterCommandUnavailable;
use Closure;
use Illuminate\Database\QueryException;
use InvalidArgumentException;
use JsonException;
use Symfony\Component\HttpFoundation\Response;

final class StoreDeliveryResponse
{
    /** @param Closure(): string $action */
    public function handle(Closure $action): Response
    {
        try {
            return response($action(), 200, ['Content-Type' => 'application/json']);
        } catch (StoreDeliveryRefused $refused) {
            return response('', $refused->status);
        } catch (InvalidArgumentException) {
            return response('', 400);
        } catch (CharacterCommandUnavailable|QueryException|JsonException) {
            return response('', 503);
        }
    }
}
