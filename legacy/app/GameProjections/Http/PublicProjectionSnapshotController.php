<?php

namespace App\GameProjections\Http;

use App\GameProjections\PublicProjectionFamily;
use App\GameProjections\PublicProjectionIngestion;
use App\GameProjections\PublicProjectionRefused;
use App\GameProjections\PublicProjectionSettings;
use App\GameProjections\PublicProjectionSnapshot;
use Illuminate\Database\QueryException;
use Illuminate\Http\Request;
use InvalidArgumentException;
use JsonException;
use Symfony\Component\HttpFoundation\Response;
use UnexpectedValueException;

/** `POST /internal/v1/game-public/{family}`: one snapshot of one subject (§4.1-§4.6). */
final class PublicProjectionSnapshotController
{
    public function __invoke(Request $request, PublicProjectionIngestion $ingestion): Response
    {
        $settings = PublicProjectionSettings::current();
        $identity = $request->server('SSL_CLIENT_S_DN');
        $route = $request->route('family');
        $family = is_string($route) ? PublicProjectionFamily::tryFrom($route) : null;
        if ($settings === null || ! is_string($identity) || $family === null) {
            return response('', 503);
        }

        try {
            $result = $ingestion->snapshot(
                $settings,
                $identity,
                PublicProjectionSnapshot::fromWire($family, $request->getContent()),
                now()->getTimestamp(),
            );
        } catch (InvalidArgumentException|JsonException) {
            return response('', 400);
        } catch (PublicProjectionRefused $refused) {
            return response('', $refused->status);
        } catch (QueryException|UnexpectedValueException) {
            return response('', 503);
        }

        return response()->json(['contract_version' => 1, 'result' => $result]);
    }
}
