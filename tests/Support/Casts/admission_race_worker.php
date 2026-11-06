<?php

use App\Casts\CastControls;
use App\Casts\CastUnavailable;
use App\Identity\Models\Identity;
use Illuminate\Contracts\Console\Kernel;
use Illuminate\Support\Carbon;
use Illuminate\Support\Facades\DB;

require dirname(__DIR__, 3).'/vendor/autoload.php';
$app = require dirname(__DIR__, 3).'/bootstrap/app.php';
$app->make(Kernel::class)->bootstrap();
$input = json_decode(stream_get_contents(STDIN), true, 8, JSON_THROW_ON_ERROR);
Carbon::setTestNow($input['now']);
DB::statement("SET application_name = 's17_cast_race'");
$viewer = Identity::query()->findOrFail($input['viewer']);
try {
    app(CastControls::class)->reserve($viewer, $input['cast'], null, '127.0.0.1');
    echo "ADMITTED\n";
} catch (CastUnavailable) {
    echo "DENIED\n";
}
