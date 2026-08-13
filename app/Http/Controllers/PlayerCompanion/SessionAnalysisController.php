<?php

namespace App\Http\Controllers\PlayerCompanion;

use App\Http\Requests\PlayerCompanion\StoreSessionAnalysisRequest;
use App\Identity\Models\Identity;
use App\PlayerCompanion\Models\SessionAnalysis;
use App\PlayerCompanion\SessionAnalysis\SessionLogParser;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;
use Illuminate\Http\Response;
use InvalidArgumentException;

final class SessionAnalysisController
{
    public function index(Request $request): Response
    {
        $identity = $this->identity($request);
        $analyses = SessionAnalysis::query()
            ->where('identity_id', $identity->id)
            ->latest('id')
            ->limit(50)
            ->get();

        return response()
            ->view('player-companion.session-analyses.index', ['analyses' => $analyses])
            ->header('Cache-Control', 'private, no-store');
    }

    public function store(StoreSessionAnalysisRequest $request, SessionLogParser $parser): RedirectResponse
    {
        $identity = $this->identity($request);
        $validated = $request->validated();

        try {
            $parsed = $parser->parse($validated['session_log']);
        } catch (InvalidArgumentException $exception) {
            return redirect()
                ->route('player-companion.session-analyses.index')
                ->withErrors(['session_log' => __('player_companion.parser_errors.'.$exception->getMessage())])
                ->withInput(['label' => $validated['label'] ?? null]);
        }

        $analysis = SessionAnalysis::query()->create([
            'identity_id' => $identity->id,
            'label' => $validated['label'] ?? null,
            ...$parsed,
        ]);

        return redirect()
            ->route('player-companion.session-analyses.show', $analysis->id)
            ->with('status', __('player_companion.saved'));
    }

    public function show(Request $request, int $analysis): Response
    {
        $identity = $this->identity($request);
        $sessionAnalysis = SessionAnalysis::query()
            ->where('identity_id', $identity->id)
            ->whereKey($analysis)
            ->firstOrFail();

        return response()
            ->view('player-companion.session-analyses.show', ['analysis' => $sessionAnalysis])
            ->header('Cache-Control', 'private, no-store');
    }

    public function destroy(Request $request, int $analysis): RedirectResponse
    {
        $identity = $this->identity($request);
        $sessionAnalysis = SessionAnalysis::query()
            ->where('identity_id', $identity->id)
            ->whereKey($analysis)
            ->firstOrFail();

        $sessionAnalysis->delete();

        return redirect()
            ->route('player-companion.session-analyses.index')
            ->with('status', __('player_companion.deleted'));
    }

    private function identity(Request $request): Identity
    {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        return $identity;
    }
}
