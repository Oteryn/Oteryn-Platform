<?php

namespace App\GameAuth\NativeLogin;

use Illuminate\Database\Eloquent\Model;

/**
 * Durable attempt_ref record (contract §6.1). Holds the exact signing input, never a signature or token.
 *
 * @property int $id
 * @property string $attempt_ref
 * @property string $ticket_hash
 * @property string $account_id
 * @property string $character_id
 * @property string|null $requested_channel_id
 * @property string $world_id
 * @property string $channel_id
 * @property string $offer_digest
 * @property string|null $signing_input
 * @property string $key_id
 * @property int $issued_at
 * @property int $expires_at
 */
final class NativeAdmissionAttempt extends Model
{
    public $timestamps = false;

    protected $table = 'native_admission_attempts';

    /** @var list<string> */
    protected $fillable = [
        'attempt_ref',
        'ticket_hash',
        'account_id',
        'character_id',
        'requested_channel_id',
        'world_id',
        'channel_id',
        'offer_digest',
        'signing_input',
        'key_id',
        'issued_at',
        'expires_at',
    ];

    /** @var list<string> */
    protected $hidden = ['ticket_hash', 'signing_input'];

    /** @return array<string, string> */
    protected function casts(): array
    {
        return [
            'issued_at' => 'integer',
            'expires_at' => 'integer',
        ];
    }
}
