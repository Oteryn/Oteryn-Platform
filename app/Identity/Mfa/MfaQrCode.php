<?php

namespace App\Identity\Mfa;

use Endroid\QrCode\Encoding\Encoding;
use Endroid\QrCode\ErrorCorrectionLevel;
use Endroid\QrCode\QrCode;
use Endroid\QrCode\RoundBlockSizeMode;
use Endroid\QrCode\Writer\SvgWriter;
use InvalidArgumentException;

final class MfaQrCode
{
    public function dataUri(string $provisioningUri): string
    {
        if (! str_starts_with($provisioningUri, 'otpauth://totp/')) {
            throw new InvalidArgumentException('The MFA provisioning URI is invalid.');
        }

        $qrCode = new QrCode(
            data: $provisioningUri,
            encoding: new Encoding('UTF-8'),
            errorCorrectionLevel: ErrorCorrectionLevel::Medium,
            size: 280,
            margin: 12,
            roundBlockSizeMode: RoundBlockSizeMode::None,
        );

        return (new SvgWriter())->write($qrCode)->getDataUri();
    }
}
