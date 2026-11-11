<?php

namespace Tests\Feature\Identity;

use App\Identity\Localization\SetIdentityLocale;
use Tests\TestCase;

final class AccountSecurityLocaleRedirectTest extends TestCase
{
    public function test_guest_security_route_preserves_polish_locale_on_login_redirect(): void
    {
        $this->get(route('identity.account-security.show', ['locale' => 'pl']))
            ->assertRedirect(route('identity.login.create'));

        self::assertSame('pl', session()->get(SetIdentityLocale::SESSION_KEY));

        $this->get(route('identity.login.create'))
            ->assertOk()
            ->assertHeader('Content-Language', 'pl')
            ->assertSee('Zaloguj się do Oteryn Platform');
    }
}
