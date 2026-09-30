<?php

namespace App\Http\Controllers\GameAuth;

use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusRefused;
use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusSettings;
use App\GameAuth\NativeRuntimeStatus\NativeScopeAssignmentIngestion;
use App\GameAuth\NativeRuntimeStatus\NativeScopeAssignmentReport;
use App\GameAuth\NativeRuntimeStatus\NativeScopeAssignmentSettings;
use Illuminate\Database\QueryException;
use Illuminate\Http\Request;
use InvalidArgumentException;
use Symfony\Component\HttpFoundation\Response;
use UnexpectedValueException;

/**
 * `POST /internal/v1/game-auth/native-scope-assignments` (Game `oteryn-game-native-runtime-status-v1` §5).
 * Same envelope and status set as the runtime report (§4): `accepted` or `superseded`, empty failures.
 * The node identity is checked against the runtime-status configuration, so both must be valid (503).
 */
final class NativeScopeAssignmentController
{
    public function __invoke(Request $request, NativeScopeAssignmentIngestion $ingestion): Response
    {
        $settings = NativeScopeAssignmentSettings::current();
        $runtime = NativeRuntimeStatusSettings::current();
        $identity = $request->server('SSL_CLIENT_S_DN');
        if ($settings === null || $runtime === null || ! is_string($identity)) {
            return response('', 503);
        }

        try {
            $result = $ingestion->ingest($settings, $runtime, $identity, NativeScopeAssignmentReport::fromWire($request->getContent()));
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
