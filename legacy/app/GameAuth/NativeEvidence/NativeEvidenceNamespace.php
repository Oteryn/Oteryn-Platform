<?php

namespace App\GameAuth\NativeEvidence;

final class NativeEvidenceNamespace
{
    public static function accountSource(string $accountId): string
    {
        return hash('sha256', "account-security\0{$accountId}");
    }

    public static function accountState(string $accountId): string
    {
        return hash('sha256', "account-security-generation\0{$accountId}");
    }

    public static function trustSource(string $issuer, string $profile, string $keyPurpose): string
    {
        return hash('sha256', "signing-trust\0{$issuer}\0{$profile}\0{$keyPurpose}");
    }

    public static function trustState(string $issuer, string $profile, string $keyPurpose): string
    {
        return hash('sha256', "signing-trust-state\0{$issuer}\0{$profile}\0{$keyPurpose}");
    }
}
