<?php

namespace App\Wiki\Http\Admin;

use App\Identity\Models\Identity;
use App\Wiki\Application\WikiArticleService;
use App\Wiki\Http\Admin\Requests\AdminWikiVersionRequest;
use App\Wiki\Infrastructure\Models\WikiArticle;
use DomainException;
use Illuminate\Http\RedirectResponse;
use Symfony\Component\HttpKernel\Exception\ConflictHttpException;

final class AdminWikiLifecycleController
{
    public function submitReview(
        AdminWikiVersionRequest $request,
        WikiArticle $article,
        WikiArticleService $articles,
    ): RedirectResponse {
        return $this->transition(
            $request,
            $article,
            static fn (Identity $actor, WikiArticle $target, int $version): WikiArticle => $articles->submitForReview(
                $actor,
                $target,
                $version,
            ),
            'Wiki article submitted for review.',
        );
    }

    public function returnDraft(
        AdminWikiVersionRequest $request,
        WikiArticle $article,
        WikiArticleService $articles,
    ): RedirectResponse {
        return $this->transition(
            $request,
            $article,
            static fn (Identity $actor, WikiArticle $target, int $version): WikiArticle => $articles->returnToDraft(
                $actor,
                $target,
                $version,
            ),
            'Wiki article returned to draft.',
        );
    }

    public function publish(
        AdminWikiVersionRequest $request,
        WikiArticle $article,
        WikiArticleService $articles,
    ): RedirectResponse {
        return $this->transition(
            $request,
            $article,
            static fn (Identity $actor, WikiArticle $target, int $version): WikiArticle => $articles->publish(
                $actor,
                $target,
                $version,
            ),
            'Wiki article published.',
        );
    }

    public function unpublish(
        AdminWikiVersionRequest $request,
        WikiArticle $article,
        WikiArticleService $articles,
    ): RedirectResponse {
        return $this->transition(
            $request,
            $article,
            static fn (Identity $actor, WikiArticle $target, int $version): WikiArticle => $articles->unpublish(
                $actor,
                $target,
                $version,
            ),
            'Wiki article unpublished and returned to draft.',
        );
    }

    public function archive(
        AdminWikiVersionRequest $request,
        WikiArticle $article,
        WikiArticleService $articles,
    ): RedirectResponse {
        return $this->transition(
            $request,
            $article,
            static fn (Identity $actor, WikiArticle $target, int $version): WikiArticle => $articles->archive(
                $actor,
                $target,
                $version,
            ),
            'Wiki article archived.',
        );
    }

    /**
     * @param  callable(Identity, WikiArticle, int): WikiArticle  $transition
     */
    private function transition(
        AdminWikiVersionRequest $request,
        WikiArticle $article,
        callable $transition,
        string $message,
    ): RedirectResponse {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        try {
            $saved = $transition($identity, $article, $request->integer('lock_version'));
        } catch (DomainException $exception) {
            throw new ConflictHttpException($exception->getMessage(), $exception);
        }

        return redirect()
            ->route('admin.wiki.articles.edit', $saved)
            ->with('status', $message);
    }
}
