<?php

namespace Tests\Unit\Architecture;

use PHPUnit\Framework\TestCase;
use RecursiveDirectoryIterator;
use RecursiveIteratorIterator;
use SplFileInfo;

final class PublicPortalSourceDependencyTest extends TestCase
{
    public function test_announcements_and_events_source_code_does_not_depend_on_public_portal(): void
    {
        $root = dirname(__DIR__, 3);
        $directories = [
            $root.'/app/Announcements',
            $root.'/app/Events',
            $root.'/resources/views/announcements',
            $root.'/resources/views/events',
        ];

        foreach ($directories as $directory) {
            $files = new RecursiveIteratorIterator(new RecursiveDirectoryIterator($directory));

            foreach ($files as $file) {
                if (! $file instanceof SplFileInfo || ! $file->isFile() || $file->getExtension() !== 'php') {
                    continue;
                }

                $contents = file_get_contents($file->getPathname());

                if (! is_string($contents)) {
                    self::fail('Unable to read source file '.$file->getPathname());
                }

                self::assertStringNotContainsString(
                    'App\\PublicPortal',
                    $contents,
                    $file->getPathname().' reintroduces the forbidden source -> PublicPortal dependency.',
                );
            }
        }
    }
}
