@extends('admin.layout')

@section('title', 'Support and Editorial Content')

@section('content')
    <div class="page-header">
        <p class="eyebrow">Content · Typed editorial routes</p>
        <h1>Support, rules and legal content</h1>
        <p class="muted">Each entry owns one stable public route and one fixed managed-page slug. Generic managed-page administration cannot edit these reserved entries.</p>
    </div>

    <div class="table-region" tabindex="0" aria-label="Typed editorial content table, horizontally scrollable on small screens">
        <table>
            <thead>
                <tr>
                    <th scope="col">Route</th>
                    <th scope="col">Required topics</th>
                    <th scope="col">Publication</th>
                    <th scope="col">Legal metadata</th>
                    <th scope="col">Action</th>
                </tr>
            </thead>
            <tbody>
                @foreach ($keys as $key)
                    @php($page = $pages->get($key->managedPageSlug()))
                    <tr>
                        <td>
                            <strong>{{ $key->label() }}</strong><br>
                            <span class="muted">{{ route($key->publicRouteName(), absolute: false) }}</span>
                        </td>
                        <td>{{ implode(', ', $key->expectedTopics()) }}</td>
                        <td>
                            @if ($page === null)
                                <span class="badge badge-warning">Missing</span>
                            @elseif ($page->published_at === null || $page->published_at->isFuture())
                                <span class="badge badge-warning">Unpublished</span>
                            @else
                                <span class="badge badge-success">Published</span><br>
                                <span class="muted">{{ $page->published_at->format('Y-m-d H:i') }}</span>
                            @endif
                        </td>
                        <td>
                            @if ($key->isLegal())
                                {{ $page?->legal_version ?? 'No version' }}<br>
                                <span class="muted">{{ $page?->legal_effective_date?->format('Y-m-d') ?? 'No effective date' }}</span>
                            @else
                                <span class="muted">Not applicable</span>
                            @endif
                        </td>
                        <td>
                            <a class="button button-secondary" href="{{ route('admin.support-content.edit', ['editorialPageKey' => $key->value]) }}">
                                {{ $page === null ? 'Create' : 'Edit' }}
                            </a>
                        </td>
                    </tr>
                @endforeach
            </tbody>
        </table>
    </div>
@endsection
