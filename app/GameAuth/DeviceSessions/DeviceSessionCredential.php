<?php

namespace App\GameAuth\DeviceSessions;

use Carbon\CarbonImmutable;
use Illuminate\Database\Eloquent\Model;

/**
 * @property string $token_hash
 * @property string $family_id
 * @property int $sequence
 * @property CarbonImmutable|null $consumed_at
 */
final class DeviceSessionCredential extends Model
{
    protected $table = 'native_device_session_credentials';

    protected $primaryKey = 'token_hash';

    public $incrementing = false;

    protected $keyType = 'string';

    public $timestamps = false;

    protected $guarded = ['*'];

    /** @var list<string> */
    protected $hidden = ['token_hash'];

    /** @return array<string, string> */
    protected function casts(): array
    {
        return ['sequence' => 'integer', 'consumed_at' => 'immutable_datetime'];
    }
}
