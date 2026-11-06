<?php

namespace App\Casts;

use App\Identity\Models\Identity;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;

final class CastPasswordController
{
    public function __invoke(Request $request, string $cast, CastControls $controls): JsonResponse
    {
        try {
            $owner = $request->user();
            $revision = $request->input('expectedRevision');
            $password = $request->input('password');
            if (config('casts.enabled') !== true || ! $owner instanceof Identity || ! is_int($revision)
                || ($password !== null && ! is_string($password))) {
                throw new CastUnavailable;
            }
            $next = $controls->password($owner, $cast, $revision, $password);

            return response()->json(['accessRevision' => $next], 200, ['Cache-Control' => 'no-store, private']);
        } catch (CastUnavailable) {
            return response()->json(['error' => 'Cast unavailable'], 409, ['Cache-Control' => 'no-store, private']);
        }
    }
}
