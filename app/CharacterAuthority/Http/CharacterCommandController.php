<?php

namespace App\CharacterAuthority\Http;

use App\CharacterAuthority\CharacterCommandExchange;
use App\CharacterAuthority\CharacterCommandRefused;
use App\CharacterAuthority\CharacterCommandSettings;
use App\CharacterAuthority\CharacterCommandUnavailable;
use App\CharacterAuthority\CharacterCommandWire;
use Closure;
use Illuminate\Database\QueryException;
use Illuminate\Http\Request;
use InvalidArgumentException;
use JsonException;
use Symfony\Component\HttpFoundation\Response;

/**
 * `ListPendingCharacterCommandsV1`, `ReadCharacterCommandIntentV1` and
 * `PublishCharacterCommandReceiptV1` (Game `oteryn-game-character-authority-commands-v1` §4).
 * Successful bodies are canonical JSON; every failure is an empty body.
 */
final class CharacterCommandController
{
    public function __construct(private readonly CharacterCommandExchange $exchange) {}

    public function pending(Request $request): Response
    {
        return $this->handle(
            $request,
            CharacterCommandWire::MAX_LIST_REQUEST_BYTES,
            fn (CharacterCommandSettings $settings, string $raw): string => $this->exchange->pending($settings, $raw),
        );
    }

    public function read(Request $request): Response
    {
        return $this->handle(
            $request,
            CharacterCommandWire::MAX_READ_REQUEST_BYTES,
            fn (CharacterCommandSettings $settings, string $raw): string => $this->exchange->read($settings, $raw),
        );
    }

    public function receipt(Request $request): Response
    {
        return $this->handle(
            $request,
            CharacterCommandWire::MAX_RECEIPT_REQUEST_BYTES,
            function (CharacterCommandSettings $settings, string $raw): string {
                $this->exchange->receipt($settings, $raw);

                return CharacterCommandWire::ACCEPTED_RESPONSE;
            },
        );
    }

    /** @param Closure(CharacterCommandSettings, string): string $handler */
    private function handle(Request $request, int $maxBytes, Closure $handler): Response
    {
        $settings = CharacterCommandSettings::current();
        if ($settings === null) {
            return response('', 503);
        }
        $contentLength = $request->headers->get('Content-Length');
        if ($contentLength !== null
            && (preg_match('/^(0|[1-9][0-9]{0,9})$/D', $contentLength) !== 1 || (int) $contentLength > $maxBytes)) {
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
            || $raw === '' || strlen($raw) > $maxBytes) {
            return response('', 400);
        }

        try {
            $body = $handler($settings, $raw);
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
