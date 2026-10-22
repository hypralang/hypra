# hypra/runtime

The PHP runtime for pages compiled by [Hypra](https://github.com/hypralang/hypra) with `--target php`.

```console
$ hypra build product.hypra --target php
✔ dist/product.php
✔ dist/product.sources.json
0 errors, 0 warnings
```

```php
$page = require 'dist/product.php';
$output = $page->render(['product' => $product]);

$output->html;      // the page
$output->markdown;  // for LLMs
$output->json;      // page metadata and actions
$output->webmcp;    // tool definitions for AI agents
```

`render()` takes:

| Argument | Meaning |
| --- | --- |
| `$data` | an array or object with the page's props; models, collections and `ArrayAccess` objects work |
| `strict` | throw a `DataError` naming the `.hypra` line when data is missing, instead of rendering it empty |
| `formFields` | a closure returning hidden fields for POST forms to this site, e.g. `fn () => ['_token' => $token]` |
| `alternateUrl` | where the page's Markdown is served; adds `<link rel="alternate" type="text/markdown">` |

For Laravel, use [hypra/laravel](../laravel).

## License

MIT OR Apache-2.0
