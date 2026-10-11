<?php

namespace App\GameProjections\Http;

use App\GameProjections\PublicProjectionIngestion;
use App\GameProjections\PublicProjectionRefused;
use App\GameProjections\PublicProjectionSettings;
use App\GameProjections\PublicProjectionWatermark;
use Illuminate\Database\QueryException;
use Illuminate\Http\Request;
use InvalidArgumentException;
use Symfony\Component\HttpFoundation\Response;
use UnexpectedValueException;

/** `POST /internal/v1/game-public/watermark`: liveness of one family feed (§5.1). */
final class PublicProjectionWatermarkController
{
    public function __invoke(Request $request, PublicProjectionIngestion $ingestion): Response
    {
        $settings = PublicProjectionSettings::current();
        $identity = $request->server('SSL_CLIENT_S_DN');
        if ($settings === null || ! is_string($identity)) {
            return response('', 503);
        }

        try {
            $result = $ingestion->watermark(
                $settings,
                $identity,
                PublicProjectionWatermark::fromWire($request->getContent()),
                now()->getTimestamp(),
            );
        } catch (InvalidArgumentException) {
            return response('', 400);
        } catch (PublicProjectionRefused $refused) {
            return response('', $refused->status);
        } catch (QueryException|UnexpectedValueException) {
            return response('', 503);
        }

        return response()->json(['contract_version' => 1, 'result' => $result]);
    }
}
