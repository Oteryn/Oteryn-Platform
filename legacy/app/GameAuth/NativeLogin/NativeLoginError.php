<?php

namespace App\GameAuth\NativeLogin;

/**
 * Native login error codes with their exact FND-04 mapping
 * (OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT §11.2).
 */
enum NativeLoginError: string
{
    case RequestMalformed = 'NATIVE_LOGIN_REQUEST_MALFORMED';
    case UnsupportedVersion = 'NATIVE_LOGIN_UNSUPPORTED_VERSION';
    case OfferUnsupported = 'NATIVE_LOGIN_OFFER_UNSUPPORTED';
    case TicketRejected = 'NATIVE_LOGIN_TICKET_REJECTED';
    case AccountSecurityDenied = 'NATIVE_LOGIN_ACCOUNT_SECURITY_DENIED';
    case AttemptConflict = 'NATIVE_LOGIN_ATTEMPT_CONFLICT';
    case CharacterConflict = 'NATIVE_LOGIN_CHARACTER_CONFLICT';
    case RouteUnavailable = 'NATIVE_LOGIN_ROUTE_UNAVAILABLE';
    case RateLimited = 'NATIVE_LOGIN_RATE_LIMITED';
    case ReconciliationRequired = 'ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED';
    case GrantExpired = 'NATIVE_LOGIN_GRANT_EXPIRED';
    case Unavailable = 'NATIVE_LOGIN_UNAVAILABLE';

    /** SECURITY_TERMINAL rows collapse to this public code (§11.1). */
    public const AUTHENTICATION_REQUIRED = 'NATIVE_LOGIN_AUTHENTICATION_REQUIRED';

    public function httpStatus(): int
    {
        return match ($this) {
            self::RequestMalformed, self::UnsupportedVersion => 400,
            self::TicketRejected, self::AccountSecurityDenied => 401,
            self::OfferUnsupported, self::AttemptConflict, self::CharacterConflict, self::GrantExpired => 409,
            self::RateLimited => 429,
            self::RouteUnavailable, self::ReconciliationRequired, self::Unavailable => 503,
        };
    }

    public function category(): string
    {
        return match ($this) {
            self::RequestMalformed, self::AttemptConflict => 'INVALID_INPUT',
            self::UnsupportedVersion, self::OfferUnsupported => 'UNSUPPORTED_REVISION',
            self::TicketRejected => 'AUTHENTICATION_FAILED',
            self::AccountSecurityDenied, self::GrantExpired => 'SESSION_REJECTED',
            self::CharacterConflict => 'CONFLICT',
            self::RateLimited => 'CAPACITY_EXCEEDED',
            self::RouteUnavailable, self::ReconciliationRequired, self::Unavailable => 'DEPENDENCY_UNAVAILABLE',
        };
    }

    public function progression(): string
    {
        return match ($this) {
            self::TicketRejected, self::AccountSecurityDenied => 'SECURITY_TERMINAL',
            self::RouteUnavailable, self::RateLimited, self::ReconciliationRequired, self::Unavailable => 'RETRYABLE',
            default => 'TERMINAL',
        };
    }

    public function publicClass(): string
    {
        return match ($this) {
            self::RequestMalformed, self::AttemptConflict, self::GrantExpired => 'RETRY_LOGIN',
            self::UnsupportedVersion, self::OfferUnsupported => 'CLIENT_UPDATE_REQUIRED',
            self::TicketRejected, self::AccountSecurityDenied => 'AUTHENTICATION_REQUIRED',
            self::CharacterConflict => 'SESSION_UNAVAILABLE',
            self::RouteUnavailable, self::RateLimited, self::ReconciliationRequired, self::Unavailable => 'TEMPORARILY_UNAVAILABLE',
        };
    }

    /** The code a client sees; the internal code stays in metrics and audit only. */
    public function publicCode(): string
    {
        return $this->progression() === 'SECURITY_TERMINAL' ? self::AUTHENTICATION_REQUIRED : $this->value;
    }
}
