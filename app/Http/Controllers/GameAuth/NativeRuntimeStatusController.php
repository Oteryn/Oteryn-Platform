<?php

namespace App\Http\Controllers\GameAuth;

use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusIngestion;
use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusRefused;
use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusReport;
use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusSettings;
use Illuminate\Database\QueryException;
use Illuminate\Http\Request;
use InvalidArgumentException;
use Symfony\Component\HttpFoundation\Response;
use UnexpectedValueException;

/** `POST /internal/v1/game-auth/native-runtime-status` (Game `oteryn-game-native-runtime-status-v1` §4). */
final class NativeRuntimeStatusController
{
    public function __invoke(Request $request, NativeRuntimeStatusIngestion $ingestion): Response
    {
        $settings = NativeRuntimeStatusSettings::current();
        $identity = $request->server('SSL_CLIENT_S_DN');
        if ($settings === null || ! is_string($identity)) {
            return response('', 503);
        }
        $contentType = $request->headers->get('Content-Type');
        if (! is_string($contentType) || strtolower(trim(explode(';', $contentType, 2)[0])) !== 'application/json') {
            return response('', 400);
        }

        try {
            $report = NativeRuntimeStatusReport::fromWire($request->getContent());
            $result = $ingestion->ingest($settings, $identity, $report, now()->getTimestamp());
        } catch (InvalidArgumentException) {
            return response('', 400);
        } catch (NativeRuntimeStatusRefused $refused) {
            return response('', $refused->status);
        } catch (QueryException|UnexpectedValueException) {
            return response('', 503);
        }

        return response()->json(['contract_version' => 1, 'result' => $result]);
    }
}
