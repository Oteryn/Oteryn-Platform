<?php

namespace App\Wiki\Http\Public;

use App\Wiki\Application\Media\WikiMediaRenderContextFactory;
use App\Wiki\Application\Rendering\WikiMarkdownRenderer;
use App\Wiki\Application\Search\InvalidWikiSearch;
use App\Wiki\Application\Search\WikiSearch;
use App\Wiki\Application\Search\WikiSearchPage;
use App\Wiki\Queries\Public\PublicWikiQuery;
use Illuminate\Database\QueryException;
use Illuminate\Http\Request;
use Illuminate\Http\Response as HttpResponse;
use Illuminate\View\View;
use Symfony\Component\HttpFoundation\Response;

final readonly class PublicWikiController
{
    public function __construct(
        private PublicWikiQuery $wiki,
        private WikiMarkdownRenderer $renderer,
        private WikiMediaRenderContextFactory $media,
        private WikiSearch $search,
    ) {}

    public function index(Request $request): View|HttpResponse
    {
        try {
            return view('wiki.index', [
                'wiki' => $this->wiki->home($this->locale($request)),
            ]);
        } catch (QueryException $exception) {
            return $this->unavailable($exception);
        }
    }

    public function category(Request $request, string $slug): View|HttpResponse
    {
        try {
            $category = $this->wiki->category($this->locale($request), $slug);
            if ($category === null) {
                abort(Response::HTTP_NOT_FOUND);
            }

            return view('wiki.category', ['category' => $category]);
        } catch (QueryException $exception) {
            return $this->unavailable($exception);
        }
    }

    public function article(Request $request, string $slug): View|HttpResponse
    {
        try {
            $locale = $this->locale($request);
            $article = $this->wiki->article($locale, $slug);
            if ($article === null) {
                abort(Response::HTTP_NOT_FOUND);
            }

            return view('wiki.article', [
                'article' => $article,
                'rendered' => $this->renderer->render(
                    $article->sourceMarkdown,
                    $this->media->public($article->translationId, $locale, $article->sourceMarkdown),
                ),
            ]);
        } catch (QueryException $exception) {
            return $this->unavailable($exception);
        }
    }

    public function search(Request $request): View|HttpResponse
    {
        $query = $request->query('q', '');
        $query = is_string($query) ? $query : '';
        $page = filter_var($request->query('page', 1), FILTER_VALIDATE_INT);
        $page = is_int($page) ? $page : 1;

        try {
            return view('wiki.search', [
                'results' => $this->search->search($this->locale($request), $query, $page),
                'searchError' => null,
            ]);
        } catch (InvalidWikiSearch $exception) {
            return response()->view('wiki.search', [
                'results' => new WikiSearchPage(trim($query), [], 1, 12, 0),
                'searchError' => __('public.wiki.search_errors.'.$exception->reason->value),
            ], Response::HTTP_UNPROCESSABLE_ENTITY);
        } catch (QueryException $exception) {
            return $this->unavailable($exception);
        }
    }

    private function locale(Request $request): string
    {
        $locale = $request->route('locale', app()->getLocale());

        return is_string($locale) ? $locale : app()->getLocale();
    }

    private function unavailable(QueryException $exception): HttpResponse
    {
        report($exception);

        return response()->view('wiki.unavailable', [], Response::HTTP_SERVICE_UNAVAILABLE);
    }
}
