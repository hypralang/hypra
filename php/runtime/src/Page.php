<?php

declare(strict_types=1);

namespace Hypra\Runtime;

/**
 * A compiled page. Generated files return an instance of a subclass:
 *
 *     $page = require 'product.php';
 *     $output = $page->render(['product' => $product]);
 */
abstract class Page
{
    /** @var array<string, string> Data the page expects, from its `props(...)`, with where each is declared. */
    protected const PROPS = [];

    /** @var list<string> The `.hypra` files this page was compiled from, the page first. */
    protected const SOURCES = [];

    abstract protected function body(Context $c): void;

    /** @return array{title: mixed, description: mixed, lang: mixed, canonical: mixed} */
    abstract protected function meta(Context $c): array;

    /**
     * @param bool $strict Throw a DataError for missing props and fields instead of rendering them empty.
     * @param (\Closure(): array<string, string>)|null $formFields Hidden fields for POST forms to this site,
     *                                                     e.g. a CSRF token.
     * @param string|null $alternateUrl Where the page's Markdown is served, for `<link rel="alternate">`.
     */
    public function render(
        array|object $data = [],
        bool $strict = false,
        ?\Closure $formFields = null,
        ?string $alternateUrl = null,
    ): Output
    {
        if ($strict) {
            foreach (static::PROPS as $prop => $at) {
                if (!Value::has($data, $prop)) {
                    throw new DataError("{$at}: prop `{$prop}` is missing from the data");
                }
            }
        }
        $c = new Context($data, $strict, $formFields, $alternateUrl);
        $this->body($c);

        return Output::assemble($c, $this->meta($c));
    }

    /** @return list<string> */
    public function props(): array
    {
        return array_keys(static::PROPS);
    }

    /** @return list<string> */
    public function sources(): array
    {
        return static::SOURCES;
    }
}
