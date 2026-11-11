<?php

namespace App\CharacterAuthority\Restore\Http;

use App\CharacterAuthority\CharacterCommandRefused;
use App\CharacterAuthority\CharacterCommandUnavailable;
use App\CharacterAuthority\Restore\GameRestoreNoticeSettings;
use App\CharacterAuthority\Restore\GameRestoreNoticeWire;
use App\CharacterAuthority\Restore\GameRestoreReconciler;
use Illuminate\Database\QueryException;
use Illuminate\Http\Request;
use InvalidArgumentException;
use JsonException;
use Symfony\Component\HttpFoundation\Response;

/**
 * `PublishGameRestoreNoticeV1`: Game delivers its restore notice and repeats the identical bytes
 * until the answer is ACKNOWLEDGED. Successful bodies are canonical JSON; failures are empty.
 */
final class GameRestoreNoticeController
{
    public function __construct(private readonly GameRestoreReconciler $reconciler) {}

    public function __invoke(Request $request): Response
    {
        $settings = GameRestoreNoticeSettings::current();
        if ($settings === null) {
            return response('', 503);
        }
        $contentLength = $request->headers->get('Content-Length');
        if ($contentLength !== null
            && (preg_match('/^(0|[1-9][0-9]{0,9})$/D', $contentLength) !== 1 || (int) $contentLength > GameRestoreNoticeWire::MAX_REQUEST_BYTES)) {
            return response('', 413);
        }
        $encoding = $request->headers->get('Content-Encoding');
        if ($request->headers->has('Transfer-Encoding')
            || (is_string($encoding) && strtolower(trim($encoding)) !== 'identity')) {
            return response('', 400);
        }
        $contentType = $request->headers->get('Content-Type');
        $raw = $request->getContent();
        if (! is_string($contentType) || strtolower(trim(explode(';', $contentType, 2)[0])) !== 'application/json'
            || $raw === '' || strlen($raw) > GameRestoreNoticeWire::MAX_REQUEST_BYTES) {
            return response('', 400);
        }

        try {
            $body = $this->reconciler->notice($settings, $raw);
        } catch (InvalidArgumentException) {
            return response('', 400);
        } catch (CharacterCommandRefused $refused) {
            return response('', $refused->status);
        } catch (CharacterCommandUnavailable|QueryException|JsonException) {
            return response('', 503);
        }

        return response($body, 200, ['Content-Type' => 'application/json']);
    }
}
