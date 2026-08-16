<?php

namespace App\Downloads;

final class DownloadCatalog
{
    public const CHANNEL_STABLE = 'stable';

    public const CHANNEL_BETA = 'beta';

    public const PLATFORM_WINDOWS = 'windows';

    public const PLATFORM_LINUX = 'linux';

    public const PLATFORM_MACOS = 'macos';

    public const ARCHITECTURE_X86_64 = 'x86_64';

    public const ARCHITECTURE_ARM64 = 'arm64';

    public const ARCHITECTURE_X86 = 'x86';

    public const UPDATE_MODE_OPTIONAL = 'optional';

    public const UPDATE_MODE_RECOMMENDED = 'recommended';

    public const UPDATE_MODE_REQUIRED = 'required';

    public const ROLLBACK_NONE = 'none';

    public const ROLLBACK_EXPLICIT = 'explicit';

    /**
     * @return list<string>
     */
    public static function channels(): array
    {
        return [self::CHANNEL_STABLE, self::CHANNEL_BETA];
    }

    /**
     * @return list<string>
     */
    public static function platforms(): array
    {
        return [self::PLATFORM_WINDOWS, self::PLATFORM_LINUX, self::PLATFORM_MACOS];
    }

    /**
     * @return list<string>
     */
    public static function architectures(): array
    {
        return [self::ARCHITECTURE_X86_64, self::ARCHITECTURE_ARM64, self::ARCHITECTURE_X86];
    }

    /**
     * @return list<string>
     */
    public static function updateModes(): array
    {
        return [
            self::UPDATE_MODE_OPTIONAL,
            self::UPDATE_MODE_RECOMMENDED,
            self::UPDATE_MODE_REQUIRED,
        ];
    }

    /**
     * @return list<string>
     */
    public static function rollbackAuthorizations(): array
    {
        return [self::ROLLBACK_NONE, self::ROLLBACK_EXPLICIT];
    }

    public static function channelLabel(string $channel): string
    {
        return match ($channel) {
            self::CHANNEL_STABLE => 'Stable',
            self::CHANNEL_BETA => 'Beta',
            default => $channel,
        };
    }

    public static function platformLabel(string $platform): string
    {
        return match ($platform) {
            self::PLATFORM_WINDOWS => 'Windows',
            self::PLATFORM_LINUX => 'Linux',
            self::PLATFORM_MACOS => 'macOS',
            default => $platform,
        };
    }

    public static function architectureLabel(string $architecture): string
    {
        return match ($architecture) {
            self::ARCHITECTURE_X86_64 => 'x86-64',
            self::ARCHITECTURE_ARM64 => 'ARM64',
            self::ARCHITECTURE_X86 => 'x86',
            default => $architecture,
        };
    }

    public static function updateModeLabel(string $mode): string
    {
        return match ($mode) {
            self::UPDATE_MODE_OPTIONAL => 'Optional',
            self::UPDATE_MODE_RECOMMENDED => 'Recommended',
            self::UPDATE_MODE_REQUIRED => 'Required',
            default => $mode,
        };
    }

    public static function formatBytes(int $bytes): string
    {
        $units = ['B', 'KB', 'MB', 'GB', 'TB'];
        $unitIndex = 0;
        $value = (float) $bytes;
        $lastUnitIndex = count($units) - 1;

        while ($value >= 1024 && $unitIndex < $lastUnitIndex) {
            $value /= 1024;
            $unitIndex++;
        }

        $decimals = $unitIndex === 0 || $value >= 100 ? 0 : 1;

        return number_format($value, $decimals).' '.$units[$unitIndex];
    }
}