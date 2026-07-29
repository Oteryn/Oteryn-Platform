<?php

namespace Tests\Unit\GameCatalog;

use App\GameCatalog\Infrastructure\Json\DuplicateJsonKeyDetector;
use PHPUnit\Framework\TestCase;

final class DuplicateJsonKeyDetectorTest extends TestCase
{
    public function test_it_reports_duplicate_keys_with_nested_paths(): void
    {
        $detector = new DuplicateJsonKeyDetector;

        self::assertSame(
            ['$.snapshot.entity_count', '$.entities[0].data.name'],
            $detector->find('{"snapshot":{"entity_count":1,"entity_count":2},"entities":[{"data":{"name":"A","name":"B"}}]}'),
        );
    }

    public function test_it_accepts_escaped_strings_numbers_arrays_and_literals(): void
    {
        $detector = new DuplicateJsonKeyDetector;

        self::assertSame([], $detector->find('{"a":"escaped \\" value","b":[-1,0,1.5,2e3,true,false,null],"c":{"unicode":"\\u0041"}}'));
    }
}
