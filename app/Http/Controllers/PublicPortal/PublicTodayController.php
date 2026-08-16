<?php

namespace App\Http\Controllers\PublicPortal;

use App\Http\Controllers\Controller;
use App\PublicPortal\Today\TodayPageQuery;
use Illuminate\Http\Request;
use Illuminate\Http\Response;

final class PublicTodayController extends Controller
{
    private const ACCEPTANCE_SCENARIO_HEADER = 'X-Oteryn-Acceptance-Today-Scenario';

    public function __construct(private readonly TodayPageQuery $today) {}

    public function __invoke(Request $request): Response
    {
        $scenario = null;
        if (app()->environment('acceptance')) {
            $candidate = $request->header(self::ACCEPTANCE_SCENARIO_HEADER);
            if (is_string($candidate) && in_array($candidate, ['empty', 'news-outage'], true)) {
                $scenario = $candidate;
            }
        }

        return response()->view('public.today.index', [
            'today' => $this->today->get(validationScenario: $scenario),
        ])->header('Cache-Control', 'no-store, max-age=0')
            ->header('Pragma', 'no-cache');
    }
}
