<?php

declare(strict_types=1);

namespace Hypra\Laravel;

use Illuminate\Http\Request;

/** The representation of a page a client asked for. */
enum Format: string
{
    case Html = 'html';
    case Markdown = 'markdown';
    case Json = 'json';

    /** Picks the first type in the Accept header that a page can be rendered as. */
    public static function fromRequest(Request $request): self
    {
        foreach ($request->getAcceptableContentTypes() as $type) {
            $format = match ($type) {
                'text/html', 'application/xhtml+xml', '*/*' => self::Html,
                'text/markdown' => self::Markdown,
                'application/json' => self::Json,
                default => null,
            };
            if ($format !== null) {
                return $format;
            }
        }

        return self::Html;
    }

    public function contentType(): string
    {
        return match ($this) {
            self::Html => 'text/html; charset=UTF-8',
            self::Markdown => 'text/markdown; charset=UTF-8',
            self::Json => 'application/json',
        };
    }
}
