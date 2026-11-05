<?php

namespace App\Http\Controllers\GameAuth;

use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusRefused;
use App\GameAuth\NativeRuntimeStatus\NativeScopeAssignmentIngestion;
use App\GameAuth\NativeRuntimeStatus\NativeScopeAssignmentSettings;
use App\GameAuth\NativeRuntimeStatus\NativeScopeRevocationReport;
use Illuminate\Database\QueryException;
use Illuminate\Http\Request;
use InvalidArgumentException;
use Symfony\Component\HttpFoundation\Response;
use UnexpectedValueException;

/** `POST /internal/v1/game-auth/native-scope-revocations` (Game contract §16.1). */
final class NativeScopeRevocationController
{
    public function __invoke(Request $request, NativeScopeAssignmentIngestion $ingestion): Response
    {
        $settings = NativeScopeAssignmentSettings::current();
        $identity = $request->server('SSL_CLIENT_S_DN');
        if ($settings === null || ! is_string($identity)) {
            return response('', 503);
        }

        try {
            $result = $ingestion->revoke($settings, $identity, NativeScopeRevocationReport::fromWire($request->getContent()));
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
