<?php

namespace App\Providers;

use App\Accounts\Contracts\CanaryAccountProvisioningGateway;
use App\CanaryIntegration\CanaryAccountProvisioner;
use App\CanaryIntegration\CanaryCharacterCreator;
use App\CanaryIntegration\CanaryCharacterTransfer;
use App\Characters\Contracts\CanaryCharacterCreationGateway;
use App\GameAuth\OAuth\RequirePublicClientPkceS256;
use App\Identity\Mfa\PendingMfaLogin;
use App\Identity\Support\CanonicalEmail;
use App\Marketplace\Contracts\CanaryCharacterTransferGateway;
use App\Payments\Contracts\PaymentProviderGateway;
use App\Payments\Contracts\PaymentWebhookVerifier;
use App\Payments\Infrastructure\DeterministicTestPaymentProvider;
use App\Wiki\Application\Rendering\WikiMarkdownRenderer;
use App\Wiki\Application\Search\WikiSearch;
use App\Wiki\Infrastructure\Rendering\CommonMarkWikiRenderer;
use App\Wiki\Infrastructure\Search\DatabaseWikiSearch;
use App\Wiki\Queries\Public\DatabasePublicWikiQuery;
use App\Wiki\Queries\Public\PublicWikiQuery;
use Illuminate\Cache\RateLimiting\Limit;
use Illuminate\Foundation\Events\LocaleUpdated;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\Event;
use Illuminate\Support\Facades\RateLimiter;
use Illuminate\Support\Facades\Route;
use Illuminate\Support\Facades\URL;
use Illuminate\Support\ServiceProvider;
use Laravel\Passport\Passport;
use LogicException;

class AppServiceProvider extends ServiceProvider
{
    public function register(): void
    {
        $this->app->bind(CanaryAccountProvisioningGateway::class, CanaryAccountProvisioner::class);
        $this->app->bind(CanaryCharacterCreationGateway::class, CanaryCharacterCreator::class);
        $this->app->bind(CanaryCharacterTransferGateway::class, CanaryCharacterTransfer::class);
        $this->app->bind(
            PaymentProviderGateway::class,
            fn (): PaymentProviderGateway => $this->deterministicTestPaymentProvider(),
        );
        $this->app->bind(
            PaymentWebhookVerifier::class,
            fn (): PaymentWebhookVerifier => $this->deterministicTestPaymentProvider(),
        );
        $this->app->bind(WikiMarkdownRenderer::class, CommonMarkWikiRenderer::class);
        $this->app->bind(WikiSearch::class, DatabaseWikiSearch::class);
        $this->app->bind(PublicWikiQuery::class, DatabasePublicWikiQuery::class);
    }

    public function boot(): void
    {
        $this->configureLocalization();
        $this->configureRateLimiters();
        $this->configureNativeOAuth();
    }

    private function deterministicTestPaymentProvider(): DeterministicTestPaymentProvider
    {
        if (config('payments.provider') !== DeterministicTestPaymentProvider::PROVIDER) {
            throw new LogicException('No approved payment provider adapter is bound.');
        }

        $secret = config('payments.webhook.test_secret');
        $maximumPayloadBytes = config('payments.webhook.maximum_payload_bytes');
        $signatureToleranceSeconds = config('payments.webhook.signature_tolerance_seconds');

        if (! is_string($secret)
            || ! is_int($maximumPayloadBytes)
            || ! is_int($signatureToleranceSeconds)) {
            throw new LogicException('The deterministic test payment provider is not configured.');
        }

        return new DeterministicTestPaymentProvider(
            $secret,
            $maximumPayloadBytes,
            $signatureToleranceSeconds,
        );
    }

    private function configureLocalization(): void
    {
        Event::listen(LocaleUpdated::class, static function (LocaleUpdated $event): void {
            URL::defaults(['locale' => $event->locale]);
        });
    }

    private function configureNativeOAuth(): void
    {
        Passport::authorizationView('game-auth.oauth.authorize');
        Passport::tokensCan([
            'game:ticket' => 'Request a one-time Oteryn game login ticket.',
        ]);
        Passport::tokensExpireIn(now()->addMinutes(
            $this->boundedPositiveInt('game-auth.oauth.access_token_ttl_minutes', 30),
        ));
        Passport::refreshTokensExpireIn(now()->addMinutes(
            $this->boundedPositiveInt('game-auth.oauth.refresh_token_ttl_minutes', 60),
        ));

        $this->app->booted(function (): void {
            $authorizationRoute = Route::getRoutes()->getByName('passport.authorizations.authorize');

            if ($authorizationRoute === null) {
                throw new LogicException('Passport authorization route is not registered.');
            }

            $authorizationRoute->middleware(RequirePublicClientPkceS256::class);
        });
    }

    private function configureRateLimiters(): void
    {
        RateLimiter::for('identity-registration', function (Request $request): Limit {
            return Limit::perMinute(5)->by($request->ip() ?? 'unknown');
        });

        RateLimiter::for('identity-login', function (Request $request): Limit {
            return Limit::perMinute(5)->by($this->emailSourceKey($request));
        });

        RateLimiter::for('identity-login-source', function (Request $request): Limit {
            return Limit::perMinute(20)->by($request->ip() ?? 'unknown');
        });

        RateLimiter::for('identity-password-recovery', function (Request $request): Limit {
            return Limit::perMinute(3)->by($this->emailSourceKey($request));
        });

        RateLimiter::for('identity-password-recovery-source', function (Request $request): Limit {
            return Limit::perMinute(10)->by($request->ip() ?? 'unknown');
        });

        RateLimiter::for('identity-password-reset', function (Request $request): Limit {
            return Limit::perMinute(5)->by($this->emailSourceKey($request));
        });

        RateLimiter::for('identity-password-change', function (Request $request): Limit {
            return Limit::perMinute(5)->by($this->authenticatedIdentitySourceKey($request));
        });

        RateLimiter::for('identity-email-change', function (Request $request): Limit {
            return Limit::perHour(3)->by($this->authenticatedIdentitySourceKey($request));
        });

        RateLimiter::for('identity-email-token', function (Request $request): Limit {
            return Limit::perMinute(10)->by($this->routeTokenSourceKey($request));
        });

        RateLimiter::for('identity-session-revoke', function (Request $request): Limit {
            return Limit::perMinute(10)->by($this->authenticatedIdentitySourceKey($request));
        });

        RateLimiter::for('identity-security-mutation', function (Request $request): Limit {
            return Limit::perMinute(10)->by($this->authenticatedIdentitySourceKey($request));
        });

        RateLimiter::for('identity-recovery-key-manage', function (Request $request): Limit {
            return Limit::perHour(5)->by($this->authenticatedIdentitySourceKey($request));
        });

        RateLimiter::for('identity-recovery-key-use', function (Request $request): Limit {
            return Limit::perHour(5)->by($this->emailSourceKey($request));
        });

        RateLimiter::for('identity-termination', function (Request $request): Limit {
            return Limit::perHour(3)->by($this->authenticatedIdentitySourceKey($request));
        });

        RateLimiter::for('identity-mfa-challenge', function (Request $request): Limit {
            $sourceIp = $request->ip() ?? 'unknown';

            return Limit::perMinute(5)->by($this->pendingMfaIdentityKey($request).'|'.$sourceIp);
        });

        RateLimiter::for('identity-mfa-challenge-identity', function (Request $request): Limit {
            return Limit::perMinute(10)->by($this->pendingMfaIdentityKey($request));
        });

        RateLimiter::for('identity-mfa-challenge-source', function (Request $request): Limit {
            return Limit::perMinute(20)->by($request->ip() ?? 'unknown');
        });

        RateLimiter::for('identity-mfa-enrollment', function (Request $request): Limit {
            return Limit::perMinute(5)->by($this->authenticatedIdentitySourceKey($request));
        });

        RateLimiter::for('identity-mfa-disable', function (Request $request): Limit {
            return Limit::perMinute(5)->by($this->authenticatedIdentitySourceKey($request));
        });

        RateLimiter::for('character-create', function (Request $request): Limit {
            return Limit::perMinute(5)->by($this->authenticatedIdentitySourceKey($request));
        });

        RateLimiter::for('marketplace-listing', function (Request $request): Limit {
            return Limit::perMinute(3)->by($this->authenticatedIdentitySourceKey($request));
        });

        RateLimiter::for('marketplace-bid', function (Request $request): Limit {
            return Limit::perMinute(20)->by($this->authenticatedIdentitySourceKey($request));
        });

        RateLimiter::for('marketplace-watch', function (Request $request): Limit {
            return Limit::perMinute(30)->by($this->authenticatedIdentitySourceKey($request));
        });

        RateLimiter::for('support-ticket-create', function (Request $request): Limit {
            return Limit::perMinute(6)->by($this->authenticatedIdentitySourceKey($request));
        });

        RateLimiter::for('support-ticket-reply', function (Request $request): Limit {
            return Limit::perMinute(12)->by($this->authenticatedIdentitySourceKey($request));
        });

        RateLimiter::for('support-ticket-status', function (Request $request): Limit {
            return Limit::perMinute(12)->by($this->authenticatedIdentitySourceKey($request));
        });

        RateLimiter::for('support-report-submit', function (Request $request): Limit {
            return Limit::perMinute(3)->by($this->authenticatedIdentitySourceKey($request));
        });

        RateLimiter::for('support-enforcement-acknowledge', function (Request $request): Limit {
            return Limit::perMinute(12)->by($this->authenticatedIdentitySourceKey($request));
        });

        RateLimiter::for('support-enforcement-appeal', function (Request $request): Limit {
            return Limit::perMinute(3)->by($this->authenticatedIdentitySourceKey($request));
        });

        RateLimiter::for('game-auth-ticket-issue', function (Request $request): Limit {
            return Limit::perMinute(5)->by($this->bearerSourceKey($request));
        });

        RateLimiter::for('game-auth-ticket-redeem-source', function (Request $request): Limit {
            return Limit::perMinute(60)->by($request->ip() ?? 'unknown');
        });

        RateLimiter::for('game-auth-ticket-redeem', function (Request $request): Limit {
            return Limit::perMinute(60)->by($this->bearerSourceKey($request));
        });

        RateLimiter::for('wiki-search', function (Request $request): Limit {
            $locale = $request->route('locale', app()->getLocale());
            $localeKey = is_string($locale) ? $locale : 'unknown';

            return Limit::perMinute(30)->by($localeKey.'|'.($request->ip() ?? 'unknown'));
        });
    }

    private function boundedPositiveInt(string $key, int $maximum): int
    {
        $value = config($key);

        if (! is_int($value) || $value < 1 || $value > $maximum) {
            throw new LogicException("Invalid bounded integer configuration: {$key}.");
        }

        return $value;
    }

    private function emailSourceKey(Request $request): string
    {
        $email = $request->input('email');
        $canonicalEmail = is_string($email) ? CanonicalEmail::normalize($email) : '';
        $identityKey = hash('sha256', $canonicalEmail);
        $sourceIp = $request->ip() ?? 'unknown';

        return $identityKey.'|'.$sourceIp;
    }

    private function authenticatedIdentitySourceKey(Request $request): string
    {
        $identifier = $request->user()?->getAuthIdentifier();
        $identityKey = is_int($identifier) || is_string($identifier)
            ? hash('sha256', (string) $identifier)
            : 'unknown';
        $sourceIp = $request->ip() ?? 'unknown';

        return $identityKey.'|'.$sourceIp;
    }

    private function pendingMfaIdentityKey(Request $request): string
    {
        $pendingIdentityId = $request->session()->get(PendingMfaLogin::IDENTITY_ID_KEY);

        return is_int($pendingIdentityId) || is_string($pendingIdentityId)
            ? hash('sha256', (string) $pendingIdentityId)
            : 'unknown';
    }

    private function routeTokenSourceKey(Request $request): string
    {
        $token = $request->route('token');
        $tokenKey = is_string($token) && $token !== '' ? hash('sha256', $token) : 'missing';

        return $tokenKey.'|'.($request->ip() ?? 'unknown');
    }

    private function bearerSourceKey(Request $request): string
    {
        $credential = $request->bearerToken();
        $credentialKey = is_string($credential) && $credential !== ''
            ? hash('sha256', $credential)
            : 'missing';

        return $credentialKey.'|'.($request->ip() ?? 'unknown');
    }
}
