<?php

namespace Tests\Feature\Wiki;

use App\Identity\Models\Identity;
use Illuminate\Support\Facades\DB;
use RuntimeException;
use Tests\Feature\EditorialMedia\EditorialMediaTestCase;

abstract class WikiEditorialMediaTestCase extends EditorialMediaTestCase
{
    /** @param list<string> $permissions */
    protected function grantWikiPermissions(Identity $identity, array $permissions): void
    {
        $now = now();
        $roleId = DB::table('admin_roles')->insertGetId([
            'key' => 'wiki-media-role-'.$identity->id,
            'name' => 'Wiki media test role',
            'created_at' => $now,
            'updated_at' => $now,
        ]);

        foreach ($permissions as $permission) {
            $permissionId = $this->integerDatabaseValue(
                DB::table('admin_permissions')->where('key', $permission)->value('id'),
                "permission {$permission}",
            );
            DB::table('admin_role_permissions')->insert([
                'role_id' => $roleId,
                'permission_id' => $permissionId,
            ]);
        }

        DB::table('identity_admin_roles')->insert([
            'identity_id' => $identity->id,
            'role_id' => $roleId,
        ]);
    }

    /**
     * @return array{
     *     content_type: string,
     *     is_featured: string,
     *     sort_order: int,
     *     category_ids: list<int>,
     *     change_note: string,
     *     translations: array{
     *         en: array{title: string, slug: string, summary: string, source_markdown: string},
     *         pl: array{title: string, slug: string, summary: string, source_markdown: string}
     *     },
     *     lock_version?: int
     * }
     */
    protected function wikiArticlePayload(
        string $englishMarkdown,
        string $polishMarkdown,
        string $slugSuffix = '',
    ): array {
        return [
            'content_type' => 'guide',
            'is_featured' => '1',
            'sort_order' => 7,
            'category_ids' => [],
            'change_note' => 'Wiki EditorialMedia test.',
            'translations' => [
                'en' => [
                    'title' => 'Media guide'.$slugSuffix,
                    'slug' => 'media-guide'.$slugSuffix,
                    'summary' => 'Approved media guidance.',
                    'source_markdown' => $englishMarkdown,
                ],
                'pl' => [
                    'title' => 'Poradnik mediÃ³w'.$slugSuffix,
                    'slug' => 'poradnik-mediow'.$slugSuffix,
                    'summary' => 'Zatwierdzony poradnik mediÃ³w.',
                    'source_markdown' => $polishMarkdown,
                ],
            ],
        ];
    }

    protected function integerDatabaseValue(mixed $value, string $description): int
    {
        if (is_int($value)) {
            return $value;
        }

        if (is_string($value) && ctype_digit($value)) {
            return (int) $value;
        }

        throw new RuntimeException("Expected an integer-compatible {$description} id.");
    }
}
