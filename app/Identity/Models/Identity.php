<?php

namespace App\Identity\Models;

use Illuminate\Auth\Passwords\CanResetPassword as CanResetPasswordTrait;
use Illuminate\Contracts\Auth\CanResetPassword as CanResetPasswordContract;
use Illuminate\Database\Eloquent\Relations\HasMany;
use Illuminate\Database\Eloquent\Relations\HasOne;
use Illuminate\Foundation\Auth\User as Authenticatable;
use Illuminate\Notifications\Notifiable;
use Illuminate\Support\Carbon;
use Laravel\Passport\Contracts\OAuthenticatable;
use Laravel\Passport\HasApiTokens;

/**
 * @property int $id
 * @property string $email
 * @property string $password
 * @property int $web_session_generation
 * @property int $game_auth_generation
 * @property Carbon|null $disabled_at
 * @property string|null $two_factor_secret
 * @property array<int, string>|null $two_factor_recovery_codes
 * @property Carbon|null $two_factor_confirmed_at
 * @property int|null $two_factor_last_used_timestep
 * @property bool $public_account_association
 * @property bool $public_status_visible
 * @property Carbon|null $email_change_available_at
 * @property Carbon|null $termination_requested_at
 * @property Carbon|null $termination_scheduled_for
 * @property Carbon|null $terminated_at
 * @property string|null $terminated_email_hash
 */
final class Identity extends Authenticatable implements CanResetPasswordContract, OAuthenticatable
{
    use CanResetPasswordTrait;
    use HasApiTokens;
    use Notifiable;

    protected $table = 'identities';

    /**
     * @var list<string>
     */
    protected $fillable = [
        'email',
        'password',
        'public_account_association',
        'public_status_visible',
    ];

    /**
     * @var list<string>
     */
    protected $hidden = [
        'password',
        'two_factor_secret',
        'two_factor_recovery_codes',
        'terminated_email_hash',
    ];

    /** @return HasMany<IdentityWebSession, $this> */
    public function webSessions(): HasMany
    {
        return $this->hasMany(IdentityWebSession::class, 'identity_id');
    }

    /** @return HasMany<IdentityEmailChangeRequest, $this> */
    public function emailChangeRequests(): HasMany
    {
        return $this->hasMany(IdentityEmailChangeRequest::class, 'identity_id');
    }

    /** @return HasOne<IdentityRecoveryKey, $this> */
    public function recoveryKey(): HasOne
    {
        return $this->hasOne(IdentityRecoveryKey::class, 'identity_id');
    }

    public function hasConfirmedMfa(): bool
    {
        return $this->two_factor_secret !== null && $this->two_factor_confirmed_at !== null;
    }

    public function hasPendingMfaEnrollment(): bool
    {
        return $this->two_factor_secret !== null && $this->two_factor_confirmed_at === null;
    }

    public function hasPendingTermination(): bool
    {
        return $this->termination_requested_at !== null
            && $this->termination_scheduled_for !== null
            && $this->terminated_at === null;
    }

    public function isTerminated(): bool
    {
        return $this->terminated_at !== null;
    }

    /**
     * @return array<string, string>
     */
    protected function casts(): array
    {
        return [
            'web_session_generation' => 'integer',
            'game_auth_generation' => 'integer',
            'disabled_at' => 'datetime',
            'two_factor_secret' => 'encrypted',
            'two_factor_recovery_codes' => 'encrypted:array',
            'two_factor_confirmed_at' => 'datetime',
            'two_factor_last_used_timestep' => 'integer',
            'public_account_association' => 'boolean',
            'public_status_visible' => 'boolean',
            'email_change_available_at' => 'datetime',
            'termination_requested_at' => 'datetime',
            'termination_scheduled_for' => 'datetime',
            'terminated_at' => 'datetime',
        ];
    }
}
