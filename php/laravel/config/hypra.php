<?php

return [

    // The hypra compiler. By default the one `php artisan hypra:install` downloaded, else `hypra` on the PATH.
    // Only needed where pages are compiled, not where compiled pages are served.
    'binary' => env('HYPRA_BINARY'),

    // Where `php artisan hypra:install` downloads the compiler from.
    'releases' => env('HYPRA_RELEASES_URL', 'https://github.com/hypralang/hypra/releases/download'),

    // Where compiled pages are cached. By default a folder inside Laravel's compiled views,
    // so `php artisan view:clear` clears them too.
    'compiled' => env('HYPRA_COMPILED_PATH'),

    // Throw when data doesn't match a page (a missing prop or field) instead of rendering it empty.
    'strict' => (bool) env('HYPRA_STRICT', env('APP_DEBUG', false)),

    // Answer with Markdown or JSON when a client asks for it in the Accept header.
    'negotiate' => true,

];
