<?php

namespace App\GameAuth\Tickets;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Support\Carbon;

/**
 * @property int $id
 * @property string $ticket_hash
 * @property int $identity_id
 * @property int|null $canary_account_id
 * @property string|null $account_id
 * @property string $audience
 * @property int $security_generation
 * @property int|null $native_security_generation
 * @property Carbon $expires_at
 * @property Carbon|null $used_at
 * @property string|null $attempt_ref
 * @property Carbon $created_at
 */
final class GameLoginTicket extends Model
{
    public $timestamps = false;

    protected $table = 'game_login_tickets';

    /**
     * @var list<string>
     */
    protected $fillable = [
        'ticket_hash',
        'identity_id',
        'canary_account_id',
        'account_id',
        'audience',
        'security_generation',
        'native_security_generation',
        'expires_at',
        'used_at',
    ];

    /**
     * @var list<string>
     */
    protected $hidden = [
        'ticket_hash',
    ];

    /**
     * @return array<string, string>
     */
    protected function casts(): array
    {
        return [
            'identity_id' => 'integer',
            'canary_account_id' => 'integer',
            'security_generation' => 'integer',
            'native_security_generation' => 'integer',
            'expires_at' => 'datetime',
            'used_at' => 'datetime',
            'created_at' => 'datetime',
        ];
    }
}
