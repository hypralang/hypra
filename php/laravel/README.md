# hypra/laravel

[Hypra](https://github.com/hypralang/hypra) views for Laravel 11, 12 and 13.

```console
$ composer require hypra/laravel
$ php artisan hypra:install
```

`hypra:install` downloads the `hypra` compiler for this machine from the GitHub release that matches the
package version, checks it against the release's SHA-256 checksums, and puts it in `storage/hypra`, where the
package finds it. No Rust needed. To keep it after `composer update`, add it to your `composer.json`:

```json
"scripts": {
    "post-update-cmd": ["@php artisan hypra:install"]
}
```

The compiler is only needed where pages are compiled. To use another one, set `HYPRA_BINARY`.

## Deploying

`php artisan hypra:compile` compiles every page ahead of time and fails if one doesn't compile, so a broken page
stops the deploy instead of reaching a visitor. From Laravel 11.27, `php artisan optimize` runs it too, and
`optimize:clear` removes the compiled pages.

## Views

`.hypra` files live next to Blade views and are rendered with `view()` as usual:

```php
// resources/views/products/show.hypra
return view('products.show', ['product' => $product]);
```

Pages are compiled on first use and cached next to Laravel's compiled views, so `php artisan view:clear`
(or `php artisan hypra:clear`) removes them. A page is recompiled when it or any file it imports changes, and
when the `hypra` binary is replaced, e.g. by an upgrade. A page that doesn't compile throws a `CompileError` with
the compiler's message:

```text
Hypra could not compile resources/views/products/show.hypra:

error[A11Y-001]: image has no alt text
 --> resources/views/components/review-card.hypra:4:9
```

## One URL, three formats

Routes in the `web` group answer with the format the client asks for in its `Accept` header:

| Accept | Response |
| --- | --- |
| `text/html` (browsers) | the page |
| `text/markdown` | the page as Markdown, with a list of its actions |
| `application/json` | page metadata and actions |

Responses carry `Vary: Accept`, so caches keep the formats apart, and the HTML links its Markdown version at
the same URL. POST forms to this site get the CSRF token; forms posting to other sites don't.

## Actions

`action("cart.add", id: product.id) "Add to cart"` posts to `/_a/cart.add`. Connect it to a handler:

```php
Route::hypra('cart.add', [CartController::class, 'add']);
```

## Configuration

```console
$ php artisan vendor:publish --tag=hypra-config
```

| Key | Default | Meaning |
| --- | --- | --- |
| `binary` | `HYPRA_BINARY`, else the one `hypra:install` downloaded, else `hypra` on the `PATH` | the compiler |
| `releases` | `HYPRA_RELEASES_URL`, or this project's GitHub releases | where `hypra:install` downloads from |
| `compiled` | `HYPRA_COMPILED_PATH`, or `hypra` inside the compiled views folder | compiled page cache |
| `strict` | `HYPRA_STRICT` or `APP_DEBUG` | throw on data that doesn't match a page |
| `negotiate` | `true` | serve Markdown and JSON via the `Accept` header |

## License

MIT OR Apache-2.0
