<?php

namespace App\Support\Models;

use App\Identity\Models\Identity;
use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;

final class SupportTicketMessage extends Model
{
    public const AUTHOR_USER = 'user';

    public const AUTHOR_STAFF = 'staff';

    public const AUTHOR_SYSTEM = 'system';

    public const VISIBILITY_PUBLIC = 'public';

    public const VISIBILITY_INTERNAL = 'internal';

    /** @var list<string> */
    protected $fillable = [
        'support_ticket_id',
        'author_identity_id',
        'author_kind',
        'visibility',
        'body',
    ];

    /** @return BelongsTo<SupportTicket, $this> */
    public function ticket(): BelongsTo
    {
        return $this->belongsTo(SupportTicket::class, 'support_ticket_id');
    }

    /** @return BelongsTo<Identity, $this> */
    public function author(): BelongsTo
    {
        return $this->belongsTo(Identity::class, 'author_identity_id');
    }

    /** @return array<string, string> */
    protected function casts(): array
    {
        return [
            'support_ticket_id' => 'integer',
            'author_identity_id' => 'integer',
        ];
    }
}
