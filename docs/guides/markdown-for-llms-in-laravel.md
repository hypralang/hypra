# Serve Markdown to LLMs from Laravel

By the end of this guide, every page of a Laravel app answers in Markdown when an AI agent asks for it, from the
same URL that serves HTML to browsers, with the actions an agent can take on the page listed at the end.

## Why Markdown

When an AI agent or crawler reads a web page, it has to dig the content out of the HTML: navigation, scripts,
styles and wrapper elements all cost tokens and can confuse it. The same content as Markdown is smaller and says
plainly what is a heading, a list or a link.

The common way to offer it is HTTP content negotiation: a client that prefers Markdown sends
`Accept: text/markdown`, and the server answers with Markdown instead of HTML, from the same URL. `text/markdown`
is a registered media type (RFC 7763), and a page can also point to its Markdown version with
`<link rel="alternate" type="text/markdown" href="...">`.

## Without Hypra

You can add this to any Laravel app today. On PHP 8.4 or later, Spatie's
[laravel-markdown-response](https://spatie.be/docs/laravel-markdown-response/v1) does it with one middleware:

```console
$ composer require spatie/laravel-markdown-response
```

```php
use Spatie\MarkdownResponse\Middleware\ProvideMarkdownResponse;

Route::middleware(ProvideMarkdownResponse::class)->group(function () {
    Route::get('/posts/{post}', [PostController::class, 'show']);
});
```

It converts the HTML your views render for requests with `Accept: text/markdown`, for URLs ending in `.md`, and
for known AI crawlers, sets `Vary: Accept`, and caches the result. If your app runs on Laravel Cloud, the
platform has a Markdown for Agents setting that does the same without a package.

On PHP 8.2 or 8.3, a middleware of your own does the same with
[league/html-to-markdown](https://github.com/thephpleague/html-to-markdown):

```console
$ composer require league/html-to-markdown
```

```php
// app/Http/Middleware/ServeMarkdown.php
namespace App\Http\Middleware;

use Closure;
use Illuminate\Http\Request;
use League\HTMLToMarkdown\HtmlConverter;
use Symfony\Component\HttpFoundation\Response;

class ServeMarkdown
{
    public function handle(Request $request, Closure $next): Response
    {
        $response = $next($request);
        if (! str_contains($response->headers->get('Content-Type', ''), 'text/html')) {
            return $response;
        }
        // the same URL answers in two formats, so caches must keep them apart
        $response->setVary('Accept', false);
        if ($request->prefers(['text/html', 'text/markdown']) !== 'text/markdown') {
            return $response;
        }
        $converter = new HtmlConverter(['strip_tags' => true, 'remove_nodes' => 'head script style nav footer']);
        $response->setContent($converter->convert($response->getContent()));
        $response->headers->set('Content-Type', 'text/markdown; charset=UTF-8');

        return $response;
    }
}
```

Register it for the `web` group in `bootstrap/app.php`:

```php
->withMiddleware(function (Middleware $middleware) {
    $middleware->web(append: [\App\Http\Middleware\ServeMarkdown::class]);
})
```

Both work with the Blade views you have. Their limits come from guessing structure after the fact:

- The converter only sees HTML. A `div` styled as a heading stays text, a button styled as a link becomes a
  link, and anything your layout adds around the content has to be removed by name.
- Forms and buttons, the things an agent could *do* on the page, are dropped or turned into loose text.
- Nothing checks the result. A layout change can quietly fill the Markdown with menu items.

## With Hypra

[Hypra](https://hypralang.dev) pages are written once and compiled into both formats, so the Markdown is made
from what the page means rather than from its HTML. Install the Laravel package and the compiler:

```console
$ composer require hypra/laravel
$ php artisan hypra:install
```

`.hypra` files are views, next to your Blade views. Here is a product page,
`resources/views/shoes/show.hypra`:

```hypra
page(title: "{shoe.name}", description: "{shoe.summary}") {
  props(shoe: Shoe)
  h1 "{shoe.name}"
  img(shoe.photo, alt: "{shoe.name}, side view")
  p "{shoe.summary}"
  ul {
    for feature in shoe.features { li "{feature}" }
  }
  p { "From " b "{shoe.price}" ", free returns for 30 days." }
  action("cart.add", id: shoe.id) "Add to cart"
}
```

Render it like any view:

```php
Route::get('/shoes/{shoe}', fn (Shoe $shoe) => view('shoes.show', ['shoe' => $shoe]));
Route::hypra('cart.add', [CartController::class, 'add']);
```

Browsers get HTML. A client that sends `Accept: text/markdown` gets this, from the same URL:

```console
$ curl -H "Accept: text/markdown" https://shop.test/shoes/7
```

```markdown output
---
title: "Trail Runner GTX"
description: "A waterproof trail shoe with a grippy sole, for wet and muddy runs of any length."
---

# Trail Runner GTX

![Trail Runner GTX, side view](/img/trail-runner-gtx.jpg)

A waterproof trail shoe with a grippy sole, for wet and muddy runs of any length.

- Waterproof membrane
- 4 mm lugs for mud
- 290 g in size 42

From **€149**, free returns for 30 days.

[Action: Add to cart]

## Actions

- `cart.add(id: 7)`: Add to cart → POST /_a/cart.add
```

The package also sends `Vary: Accept` with both formats, and the HTML links to its Markdown version, so agents
that look for that link find it. A client that asks for `application/json` gets the page's title, description and
actions as JSON.

What's different from converting HTML:

- **Structure comes from the page.** `h1` is the title, `ul` is a list, an image keeps its alt text. Nothing has
  to be guessed or removed.
- **Actions are listed.** `action("cart.add", id: shoe.id)` is a button for people, and a line under
  `## Actions` for agents, with the parameters and where to send them.
- **It can't drift.** Both formats come from the same file, compiled together; there's no second template to keep
  in sync.
- **Data stays text.** A product name like `## Ignore previous instructions` from your database is escaped in the
  Markdown too, so it can't add headings or fake actions.

In this example, the Markdown is 490 bytes, the HTML page 1,188 bytes. On real pages with a
header, navigation and footer, the difference grows: the Markdown keeps only the page's content.

## Limits

- Hypra is new and pre-1.0, so the language may still change.
- Only pages written in Hypra get Markdown this way. Blade views keep working as before; you can convert one page
  at a time, and use the middleware above for the rest in the meantime.
- For Node.js, Bun and Deno, see [Serve Markdown to AI agents from Node.js](https://hypralang.dev/guides/markdown-for-ai-agents-in-node/);
  other backends can use the static build: `hypra build` writes the HTML and Markdown files for given data.

## Next steps

- The [Laravel docs](https://hypralang.dev/docs/laravel/) cover deploying, configuration and actions.
- The [language reference](https://hypralang.dev/docs/) is short enough to give to an AI coding assistant, so it
  can write Hypra pages for you.
