<?php

namespace App\Http\Controllers\Admin;

use Illuminate\Contracts\View\View;
use Illuminate\Support\Facades\DB;

final class AdminAuditController
{
    public function index(): View
    {
        $events = DB::table('admin_audit_events')
            ->leftJoin('identities as actor', 'actor.id', '=', 'admin_audit_events.actor_identity_id')
            ->select([
                'admin_audit_events.id',
                'admin_audit_events.actor_identity_id',
                'actor.email as actor_email',
                'admin_audit_events.action',
                'admin_audit_events.target_type',
                'admin_audit_events.target_id',
                'admin_audit_events.metadata',
                'admin_audit_events.occurred_at',
            ])
            ->orderByDesc('admin_audit_events.occurred_at')
            ->orderByDesc('admin_audit_events.id')
            ->paginate(50);

        return view('admin.audit.index', ['events' => $events]);
    }
}
