@extends('admin.layout')

@section('title', 'Editorial media')

@push('head')
    <script src="{{ asset('js/media-fallbacks.js') }}" defer></script>
@endpush

@section('content')
    <div class="page-heading">
        <div>
            <p class="eyebrow">Content</p>
            <h1>Editorial image library</h1>
            <p>Private, normalized JPEG, PNG and WebP images for approved editorial consumers.</p>
        </div>
    </div>

    <section class="panel" aria-labelledby="media-upload-heading">
        <h2 id="media-upload-heading">Upload an image</h2>
        <form method="POST" action="{{ route('admin.media.store') }}" enctype="multipart/form-data" class="form-stack">
            @csrf
            <div class="form-field">
                <label for="image">Image</label>
                <input
                    id="image"
                    name="image"
                    type="file"
                    accept="image/jpeg,image/png,image/webp,.jpg,.jpeg,.png,.webp"
                    required
                >
                <p class="field-help">JPEG, PNG or WebP only. Maximum {{ number_format(config('editorial_media.max_bytes') / 1048576, 0) }} MB.</p>
            </div>
            <div class="form-field">
                <label for="alt_text">Alternative text</label>
                <input
                    id="alt_text"
                    name="alt_text"
                    type="text"
                    maxlength="500"
                    value="{{ old('alt_text') }}"
                    required
                >
                <p class="field-help">Describe the image's editorial meaning for readers who cannot see it.</p>
            </div>
            <button class="button-primary" type="submit">Upload image</button>
        </form>
    </section>

    <section class="panel" aria-labelledby="media-library-heading">
        <div class="section-heading">
            <div>
                <h2 id="media-library-heading">Library</h2>
                <p>Deletion is disabled while any Wiki, Events or CMS reference exists.</p>
            </div>
        </div>

        @if ($mediaItems->isEmpty())
            <div class="empty-state">
                <h3>No editorial images</h3>
                <p>Upload the first approved raster image above.</p>
            </div>
        @else
            <div class="table-wrap">
                <table>
                    <thead>
                    <tr>
                        <th scope="col">Preview</th>
                        <th scope="col">Details</th>
                        <th scope="col">Integrity</th>
                        <th scope="col">References</th>
                        <th scope="col">Actions</th>
                    </tr>
                    </thead>
                    <tbody>
                    @foreach ($mediaItems as $media)
                        <tr>
                            <td>
                                <a href="{{ route('admin.media.content', $media) }}">
                                    <img
                                        src="{{ route('admin.media.thumbnail', $media) }}"
                                        alt="{{ $media->alt_text }}"
                                        width="160"
                                        height="160"
                                        loading="lazy"
                                        decoding="async"
                                        class="admin-media-preview"
                                        data-media-fallback="admin"
                                    >
                                </a>
                            </td>
                            <td>
                                <strong>{{ $media->original_name ?? 'Unnamed upload' }}</strong><br>
                                {{ $media->mime_type }} · {{ number_format($media->byte_size) }} bytes<br>
                                {{ $media->width }} × {{ $media->height }} px<br>
                                <span>{{ $media->alt_text }}</span>
                            </td>
                            <td><code class="admin-media-digest">{{ $media->sha256 }}</code></td>
                            <td>{{ $media->references_count }}</td>
                            <td>
                                @if ((int) $media->references_count === 0)
                                    <form method="POST" action="{{ route('admin.media.destroy', $media) }}">
                                        @csrf
                                        @method('DELETE')
                                        <button class="button-danger" type="submit">Delete</button>
                                    </form>
                                @else
                                    <span class="status-badge">In use</span>
                                @endif
                            </td>
                        </tr>
                    @endforeach
                    </tbody>
                </table>
            </div>

            {{ $mediaItems->links() }}
        @endif
    </section>
@endsection
