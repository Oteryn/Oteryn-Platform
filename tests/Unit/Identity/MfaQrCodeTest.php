<?php

namespace Tests\Unit\Identity;

use App\Identity\Mfa\MfaQrCode;
use InvalidArgumentException;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

final class MfaQrCodeTest extends TestCase
{
    #[Test]
    public function it_renders_an_inline_svg_without_an_external_service(): void
    {
        $dataUri = (new MfaQrCode)->dataUri(
            'otpauth://totp/Oteryn%20Platform:test%40example.com?secret=JBSWY3DPEHPK3PXP&issuer=Oteryn%20Platform',
        );

        self::assertStringStartsWith('data:image/svg+xml;base64,', $dataUri);

        $svg = base64_decode(substr($dataUri, strlen('data:image/svg+xml;base64,')), true);

        self::assertIsString($svg);
        self::assertStringContainsString('<svg', $svg);
        self::assertStringNotContainsString('otpauth://', $svg);
    }

    #[Test]
    public function it_rejects_non_totp_provisioning_uris(): void
    {
        $this->expectException(InvalidArgumentException::class);

        (new MfaQrCode)->dataUri('https://example.com/not-an-mfa-secret');
    }
}
