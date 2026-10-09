<?php

namespace App\Http\Controllers\GameAuth\DeviceSessions;

use App\GameAuth\DeviceSessions\DeviceSessionDenied;
use App\GameAuth\DeviceSessions\DeviceSessionPurpose;
use App\GameAuth\DeviceSessions\IssuedDeviceSessionCredential;
use App\GameAuth\DeviceSessions\NativeRememberedDeviceSessions;
use App\GameAuth\DeviceSessions\VerifiedRememberedDeviceAuthorization;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersAccountView;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersReadModel;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharacterSummary;
use App\GameAuth\NativeLogin\NativeGameLoginTickets;
use App\GameAuth\Tickets\GameLoginTicketDenied;
use App\Identity\Models\Identity;
use Closure;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use Laravel\Passport\AccessToken;
use RuntimeException;
use Throwable;

/** Additive candidate adapters; middleware and service both refuse default/production activation. */
final class NativeDeviceSessionController
{
    private const MAX_RESPONSE_BYTES = 16384;

    public function enroll(Request $request): JsonResponse
    {
        try {
            // Resolve the normal Passport API guard inside this protected boundary, so
            // dependency/guard failures cannot become debug responses containing credentials.
            $identity = $request->user('api');
            if (! $identity instanceof Identity) {
                throw new DeviceSessionDenied;
            }
            $token = $identity->currentAccessToken();
            if (! $token instanceof AccessToken) {
                throw new DeviceSessionDenied;
            }
            $tokenId = $token->toArray()['oauth_access_token_id'] ?? null;
            if (! is_string($tokenId) || $tokenId === '') {
                throw new DeviceSessionDenied;
            }
            $credential = app(NativeRememberedDeviceSessions::class)->enroll($identity, $tokenId, explicitConsent: true);

            return response()->json(['protocol_version' => 1] + $this->credentialPayload($credential));
        } catch (DeviceSessionDenied) {
            return $this->denied();
        } catch (Throwable) {
            return $this->unavailable();
        }
    }

    public function characters(Request $request): JsonResponse
    {
        return $this->rotate($request, DeviceSessionPurpose::NativeCharactersRead, function (VerifiedRememberedDeviceAuthorization $authorization): array {
            $view = app(NativeAccountCharactersReadModel::class)->viewForAccount($authorization->accountId, now()->getTimestamp(), lock: true);
            if (! in_array($view->state, [NativeAccountCharactersAccountView::READY, NativeAccountCharactersAccountView::MISSING], true)) {
                throw new RuntimeException('Native owner projection is unavailable.');
            }
            $payload = ['characters' => array_map(static fn (NativeAccountCharacterSummary $character): array => [
                'character_id' => $character->characterId,
                'world_id' => $character->worldId,
                'name' => $character->name,
                'availability' => $character->availability,
            ], $view->characters)];
            if (strlen(json_encode($payload, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE)) > self::MAX_RESPONSE_BYTES - 512) {
                throw new RuntimeException('Native owner projection exceeds the response bound.');
            }

            return $payload;
        });
    }

    public function ticket(Request $request): JsonResponse
    {
        return $this->rotate($request, DeviceSessionPurpose::NativeTicketIssue, function (VerifiedRememberedDeviceAuthorization $authorization): array {
            $ticket = app(NativeGameLoginTickets::class)->issue($authorization->identity());

            return [
                'ticket' => $ticket->ticket,
                'expires_in' => max(0, (int) floor(now()->diffInSeconds($ticket->expiresAt, false))),
                'expires_at' => $ticket->expiresAt->getTimestamp(),
            ];
        });
    }

    public function revoke(Request $request): JsonResponse
    {
        try {
            app(NativeRememberedDeviceSessions::class)->revokeCredential($this->secret($request), $this->clientId($request));

            return response()->json(['protocol_version' => 1, 'revoked' => true]);
        } catch (DeviceSessionDenied) {
            return $this->denied();
        } catch (Throwable) {
            return $this->unavailable();
        }
    }

    /** @param Closure(VerifiedRememberedDeviceAuthorization): array<string, mixed> $consume */
    private function rotate(Request $request, DeviceSessionPurpose $purpose, Closure $consume): JsonResponse
    {
        try {
            $result = app(NativeRememberedDeviceSessions::class)->rotateAndUse($this->secret($request), $this->clientId($request), $purpose, $consume);

            return response()->json(['protocol_version' => 1] + $this->credentialPayload($result->credential()) + $result->result());
        } catch (DeviceSessionDenied|GameLoginTicketDenied) {
            return $this->denied();
        } catch (Throwable) {
            return $this->unavailable();
        }
    }

    private function clientId(Request $request): string
    {
        $id = $request->attributes->get('native_device_client_id');
        if (! is_string($id)) {
            throw new DeviceSessionDenied;
        }

        return $id;
    }

    private function secret(Request $request): string
    {
        $header = $request->headers->get('Authorization');
        if (! is_string($header) || preg_match('/\AOterynDevice (otd1\.[A-Za-z0-9_-]{43})\z/', $header, $matches) !== 1) {
            throw new DeviceSessionDenied;
        }

        return $matches[1];
    }

    /** @return array<string, string|int> */
    private function credentialPayload(IssuedDeviceSessionCredential $credential): array
    {
        return [
            'device_credential' => $credential->secret(),
            'family_id' => $credential->familyId,
            'absolute_expires_at' => $credential->absoluteExpiresAt->getTimestamp(),
            'idle_expires_at' => $credential->idleExpiresAt->getTimestamp(),
        ];
    }

    private function denied(): JsonResponse
    {
        return response()->json(['error' => 'device_authorization_unavailable'], 401);
    }

    private function unavailable(): JsonResponse
    {
        return response()->json(['error' => 'device_authorization_unavailable'], 503);
    }
}
