@foreach ($worlds as $world)
    <section class="production-channel-summary" data-native-world-state="{{ $world->publicState() }}">
        <h3>{{ $world->name }}</h3>
        <strong>{{ __('today.cards.liveops.states.'.$world->publicState().'.label') }}</strong>
        <p>{{ __('today.cards.liveops.states.'.$world->publicState().'.summary') }}</p>
    </section>
@endforeach
