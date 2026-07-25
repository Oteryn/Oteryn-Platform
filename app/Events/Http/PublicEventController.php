<?php

namespace App\Events\Http;

use App\Events\Queries\EventCalendarQuery;
use Illuminate\Contracts\View\View;

final class PublicEventController
{
    public function __construct(private readonly EventCalendarQuery $events) {}

    public function index(): View
    {
        return view('events.index', [
            'calendar' => $this->events->calendar(app()->getLocale()),
        ]);
    }

    public function show(string $slug): View
    {
        $event = $this->events->findPublishedBySlug(app()->getLocale(), $slug);
        abort_if($event === null, 404);

        return view('events.show', ['event' => $event]);
    }
}
