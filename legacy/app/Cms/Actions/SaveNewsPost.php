<?php

namespace App\Cms\Actions;

use App\Audit\AdminAuditRecorder;
use App\Cms\Models\NewsPost;
use App\Identity\Models\Identity;
use Illuminate\Support\Facades\DB;

final class SaveNewsPost
{
    public function __construct(private readonly AdminAuditRecorder $audit) {}

    public function execute(
        Identity $actor,
        ?NewsPost $post,
        string $slug,
        string $title,
        string $body,
        ?string $publishedAt,
    ): NewsPost {
        return DB::transaction(function () use ($actor, $post, $slug, $title, $body, $publishedAt): NewsPost {
            $created = $post === null;
            $post ??= new NewsPost;
            $post->fill([
                'slug' => $slug,
                'title' => $title,
                'body' => $body,
                'published_at' => $publishedAt,
            ]);
            $post->save();

            $this->audit->record(
                $actor->id,
                $created ? 'cms.news_created' : 'cms.news_updated',
                'news_post',
                (string) $post->id,
                [
                    'slug' => $post->slug,
                    'published' => $post->published_at !== null,
                ],
            );

            return $post;
        }, 3);
    }
}
