<?php

namespace App\Http\Controllers\GameAuth;

use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersIngestion;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersRefused;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersSettings;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersWatermark;
use Illuminate\Database\QueryException;
use Illuminate\Http\Request;
use InvalidArgumentException;
use Symfony\Component\HttpFoundation\Response;
use UnexpectedValueException;

final class NativeAccountCharactersWatermarkController
{
    public function __invoke(Request $request, NativeAccountCharactersIngestion $ingestion): Response
    {
        $settings = NativeAccountCharactersSettings::current();
        $identity = $request->server('SSL_CLIENT_S_DN');
        if ($settings === null || ! is_string($identity)) {
            return response('', 503);
        }

        try {
            $result = $ingestion->watermark(
                $settings,
                $identity,
                NativeAccountCharactersWatermark::fromWire($request->getContent()),
                now()->getTimestamp(),
            );
        } catch (InvalidArgumentException) {
            return response('', 400);
        } catch (NativeAccountCharactersRefused $refused) {
            return response('', $refused->status);
        } catch (QueryException|UnexpectedValueException) {
            return response('', 503);
        }

        return response()->json(['contract_version' => 1, 'result' => $result]);
    }
}
