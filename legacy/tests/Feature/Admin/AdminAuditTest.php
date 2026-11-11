<?php

namespace Tests\Feature\Admin;

use App\Admin\AdminRoleManager;
use App\Audit\AdminAuditRecorder;
use App\Identity\Models\Identity;
use App\Identity\Sessions\WebSessionState;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Password;
use Tests\TestCase;

final class AdminAuditTest extends TestCase
{
    use RefreshDatabase;

    public function test_audit_view_requires_explicit_audit_permission(): void
    {
        $actor = $this->createIdentity('content-editor@example.com');
        $this->assignRole($actor, AdminRoleManager::CONTENT_EDITOR);
        $this->actingAsCurrent($actor);

        $this->get(route('admin.audit.index'))->assertForbidden();
    }

    public function test_audit_view_requires_confirmed_mfa(): void
    {
        $actor = $this->createIdentity('security-no-mfa@example.com', false);
        $this->assignRole($actor, AdminRoleManager::SECURITY_ADMIN);
        $this->actingAsCurrent($actor);

        $this->get(route('admin.audit.index'))->assertForbidden();
    }

    public function test_security_admin_can_view_bounded_audit_pages(): void
    {
        $actor = $this->createIdentity('security-audit@example.com');
        $this->assignRole($actor, AdminRoleManager::SECURITY_ADMIN);

        $recorder = app(AdminAuditRecorder::class);
        for ($index = 1; $index <= 51; $index++) {
            $recorder->record(
                $actor->id,
                sprintf('test.audit.%03d', $index),
                'test_target',
                (string) $index,
            );
        }

        $this->actingAsCurrent($actor);

        $this->get(route('admin.audit.index'))
            ->assertOk()
            ->assertSeeText('test.audit.051')
            ->assertDontSeeText('test.audit.001');

        $this->get(route('admin.audit.index', ['page' => 2]))
            ->assertOk()
            ->assertSeeText('test.audit.001')
            ->assertDontSeeText('test.audit.051');
    }

    public function test_privileged_audit_records_exclude_identity_and_application_secrets(): void
    {
        $plainPassword = 'Correct-Horse-9!Battery';
        $totpSecret = 'TEST-MFA-SECRET-NOT-REAL';
        $plainRecoveryCode = 'AUDIT-RECOVERY-CODE-12345';
        $recoveryCodeHash = Hash::make($plainRecoveryCode);

        $actor = $this->createIdentity('audit-secret-actor@example.com');
        $actor->forceFill([
            'two_factor_recovery_codes' => [$recoveryCodeHash],
        ])->save();
        $actor->refresh();

        $passwordHash = $actor->password;
        $resetToken = Password::createToken($actor);
        $storedResetTokenHash = DB::table('password_reset_tokens')
            ->where('email', $actor->email)
            ->value('token');
        $encryptedTotpState = DB::table('identities')
            ->where('id', $actor->id)
            ->value('two_factor_secret');
        $encryptedRecoveryCodeState = DB::table('identities')
            ->where('id', $actor->id)
            ->value('two_factor_recovery_codes');
        $applicationKey = config('app.key');

        self::assertIsString($storedResetTokenHash);
        self::assertIsString($encryptedTotpState);
        self::assertIsString($encryptedRecoveryCodeState);
        self::assertIsString($applicationKey);
        self::assertNotSame('', $applicationKey);

        $this->assignRole($actor, AdminRoleManager::PLATFORM_ADMIN);
        $target = $this->createIdentity('audit-secret-target@example.com');
        $this->actingAsCurrent($actor);

        $this->post(route('admin.roles.store', $target), [
            'role' => AdminRoleManager::CONTENT_EDITOR,
        ])->assertRedirect(route('admin.roles.index'));

        $this->post(route('admin.news.store'), [
            'slug' => 'audit-secret-regression',
            'title' => 'Audit secret regression',
            'body' => 'Audit metadata must remain bounded.',
            'published_at' => null,
        ])->assertRedirect();

        $auditPayload = json_encode(
            DB::table('admin_audit_events')->orderBy('id')->get()->all(),
            JSON_THROW_ON_ERROR,
        );

        $sensitiveValues = [
            'plain password' => $plainPassword,
            'password hash' => $passwordHash,
            'TOTP secret' => $totpSecret,
            'encrypted TOTP state' => $encryptedTotpState,
            'plain recovery code' => $plainRecoveryCode,
            'recovery-code hash' => $recoveryCodeHash,
            'encrypted recovery-code state' => $encryptedRecoveryCodeState,
            'plain reset token' => $resetToken,
            'stored reset-token hash' => $storedResetTokenHash,
            'application key' => $applicationKey,
        ];

        foreach ($sensitiveValues as $label => $sensitiveValue) {
            self::assertStringNotContainsString($sensitiveValue, $auditPayload, $label);
        }
    }

    private function createIdentity(string $email, bool $confirmedMfa = true): Identity
    {
        $identity = Identity::query()->create([
            'email' => $email,
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);

        if ($confirmedMfa) {
            $identity->forceFill([
                'two_factor_secret' => 'TEST-MFA-SECRET-NOT-REAL',
                'two_factor_confirmed_at' => now(),
            ])->save();
        }

        return $identity;
    }

    private function assignRole(Identity $identity, string $roleKey): void
    {
        $roleId = DB::table('admin_roles')->where('key', $roleKey)->value('id');

        if (! is_int($roleId) && ! (is_string($roleId) && ctype_digit($roleId))) {
            self::fail('Expected an integer-compatible administrator role id.');
        }

        DB::table('identity_admin_roles')->insert([
            'identity_id' => $identity->id,
            'role_id' => (int) $roleId,
        ]);
    }

    private function actingAsCurrent(Identity $identity): void
    {
        $currentIdentity = Identity::query()->findOrFail($identity->id);

        $this->actingAs($identity, 'web')
            ->withSession([WebSessionState::GENERATION_KEY => $currentIdentity->web_session_generation]);
    }
}
