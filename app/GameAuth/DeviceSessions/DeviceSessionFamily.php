<?php

namespace App\GameAuth\DeviceSessions;

use Carbon\CarbonImmutable;
use Illuminate\Database\Eloquent\Model;

/**
 * @property string $id
 * @property int $identity_id
 * @property string $account_id
 * @property string $oauth_client_id
 * @property string $enrollment_access_token_id
 * @property int $game_auth_generation
 * @property int $native_security_generation
 * @property int $current_sequence
 * @property CarbonImmutable $absolute_expires_at
 * @property CarbonImmutable $idle_expires_at
 * @property CarbonImmutable|null $revoked_at
 * @property string|null $revocation_reason
 */
final class DeviceSessionFamily extends Model
{
    protected $table = 'native_device_session_families';

    public $incrementing = false;

    protected $keyType = 'string';

    protected $guarded = ['*'];

    /** @var list<string> */
    protected $hidden = ['enrollment_access_token_id'];

    /** @return array<string, string> */
    protected function casts(): array
    {
        return [
            'identity_id' => 'integer',
            'game_auth_generation' => 'integer',
            'native_security_generation' => 'integer',
            'current_sequence' => 'integer',
            'absolute_expires_at' => 'immutable_datetime',
            'idle_expires_at' => 'immutable_datetime',
            'revoked_at' => 'immutable_datetime',
        ];
    }
}
