<?php

declare(strict_types=1);

namespace Hypra\Runtime;

/**
 * How values are read, shown and typed. Matches the Rust interpreter so both produce identical output.
 */
final class Value
{
    // never throw while rendering: invalid UTF-8 is replaced, NAN and INF become 0
    private const JSON_FLAGS = JSON_UNESCAPED_SLASHES
        | JSON_UNESCAPED_UNICODE
        | JSON_UNESCAPED_LINE_TERMINATORS
        | JSON_PRESERVE_ZERO_FRACTION
        | JSON_INVALID_UTF8_SUBSTITUTE
        | JSON_PARTIAL_OUTPUT_ON_ERROR;

    public static function has(mixed $value, string $key): bool
    {
        return match (true) {
            is_array($value) => array_key_exists($key, $value),
            $value instanceof \ArrayAccess => $value->offsetExists($key) || self::hasNullAttribute($value, $key),
            is_object($value) => isset($value->{$key}) || property_exists($value, $key),
            default => false,
        };
    }

    /**
     * Eloquent models report a `null` column as missing from offsetExists(); a column, loaded relation or
     * accessor that is null still exists.
     */
    private static function hasNullAttribute(object $model, string $key): bool
    {
        return (method_exists($model, 'getAttributes') && array_key_exists($key, $model->getAttributes()))
            || (method_exists($model, 'relationLoaded') && $model->relationLoaded($key))
            || (method_exists($model, 'hasGetMutator') && $model->hasGetMutator($key))
            || (method_exists($model, 'hasAttributeGetMutator') && $model->hasAttributeGetMutator($key));
    }

    public static function read(mixed $value, string $key): mixed
    {
        return match (true) {
            is_array($value) => $value[$key],
            $value instanceof \ArrayAccess => $value[$key],
            default => $value->{$key},
        };
    }

    /** The text a value shows as: strings as-is, null as nothing, everything else as JSON. */
    public static function display(mixed $value): string
    {
        return match (true) {
            $value === null => '',
            is_string($value) => $value,
            is_bool($value) => $value ? 'true' : 'false',
            is_int($value) => (string) $value,
            is_float($value) => self::json($value),
            $value instanceof \BackedEnum => self::display($value->value),
            $value instanceof \UnitEnum => $value->name,
            $value instanceof \Stringable => (string) $value,
            default => self::json($value),
        };
    }

    public static function json(mixed $value): string
    {
        // not `?:`: the JSON for 0 is "0", which PHP treats as false
        $json = json_encode($value, self::JSON_FLAGS);
        if ($json === false) {
            return 'null';
        }

        // floats print like Rust's: 1e+20, not 1.0e+20
        return is_float($value) ? str_replace('.0e', 'e', $json) : $json;
    }

    /**
     * URLs that run code when followed are replaced, so data can't turn a link into a script. Characters URLs
     * can't contain (spaces, quotes, braces, …) are percent-encoded, as a browser would when following the link.
     */
    public static function safeUrl(string $url): string
    {
        if (self::runsCode($url)) {
            return 'about:blank#blocked';
        }

        $out = '';
        foreach (str_split($url) as $byte) {
            $code = ord($byte);
            $encode = $code <= 0x20 || $code === 0x7F || str_contains('"<>\\^`{|}', $byte);
            $out .= $encode ? sprintf('%%%02X', $code) : $byte;
        }

        return $out;
    }

    /**
     * Data shown as Markdown text stays text: it can't add a heading such as a fake `## Actions`, a list, a link
     * or HTML. Line breaks become spaces, so it can only start a line where the page starts one.
     */
    public static function markdown(string $text): string
    {
        $text = addcslashes(str_replace(["\r", "\n"], ' ', $text), '\\`[]<|');

        // what the text starts with could make the line a heading, list item, rule or quote: `# `, `- `, `---`,
        // `1. `, `>`
        $lead = strspn($text, " \t");
        $at = static fn (int $i): string => $text[$lead + $i] ?? '';
        $endsMarker = static fn (int $i): bool => in_array($at($i), ['', ' ', "\t"], true);
        $digits = strspn($text, '0123456789', $lead);
        $mark = match (true) {
            $at(0) === '>' => $lead,
            in_array($at(0), ['-', '+', '*', '_'], true) && ($endsMarker(1) || $at(1) === $at(0)) => $lead,
            $at(0) === '#' && $endsMarker(strspn($text, '#', $lead)) => $lead,
            $digits > 0 && in_array($at($digits), ['.', ')'], true) && $endsMarker($digits + 1) => $lead + $digits,
            default => null,
        };

        return $mark === null ? $text : substr_replace($text, '\\', $mark, 0);
    }

    /** A Markdown link or image target: a safe URL that can't close the parentheses around it. */
    public static function markdownUrl(string $url): string
    {
        return str_replace(['(', ')'], ['%28', '%29'], self::safeUrl($url));
    }

    /** A whole Markdown code span, wider when the text holds a backtick. */
    public static function markdownSpan(string $text): string
    {
        $code = self::markdownCode(str_replace(["\r", "\n"], ' ', $text));

        return str_contains($code, '`') ? "`` {$code} ``" : "`{$code}`";
    }

    /**
     * Breaks every run of backticks with a zero-width space, so the text can't close the code span or fence around
     * it. Replacing doesn't overlap, so one pass leaves two backticks of a run of three; the second breaks them.
     */
    public static function markdownCode(string $text): string
    {
        $zwsp = "\u{200B}";

        return str_replace('``', "`{$zwsp}`", str_replace('``', "`{$zwsp}`", $text));
    }

    /** Browsers ignore whitespace and control characters in a URL scheme, so they're ignored here too. */
    public static function runsCode(string $url): bool
    {
        $scheme = '';
        foreach (str_split($url) as $byte) {
            if (ord($byte) > 0x20 && ord($byte) !== 0x7F) {
                $scheme .= strtolower($byte);
            }
        }

        return str_starts_with($scheme, 'javascript:')
            || str_starts_with($scheme, 'vbscript:')
            || str_starts_with($scheme, 'data:text/html');
    }

    /**
     * A URL on this site, the only kind that receives the page's hidden form fields such as a CSRF token.
     * Browsers drop tabs and newlines, skip leading spaces and control characters and read `\` as `/`, so this
     * does too. Then only a URL without a scheme that doesn't start with `//` is on this site.
     */
    public static function isSameSite(string $url): bool
    {
        $url = ltrim(str_replace(["\t", "\n", "\r", '\\'], ['', '', '', '/'], $url), "\x00..\x20");
        if (str_starts_with($url, '//')) {
            return false;
        }
        // a scheme ends with the first `:` that comes before any `/`, `?` or `#`
        $end = strcspn($url, ':/?#');

        return $end === strlen($url) || $url[$end] !== ':';
    }

    public static function escape(string $text): string
    {
        return strtr($text, ['&' => '&amp;', '<' => '&lt;', '>' => '&gt;', '"' => '&quot;']);
    }

    public static function truthy(mixed $value): bool
    {
        return match (true) {
            $value === null => false,
            is_bool($value) => $value,
            is_string($value) => $value !== '',
            is_int($value), is_float($value) => $value != 0,
            is_array($value) => $value !== [],
            $value instanceof \Countable => count($value) > 0,
            $value instanceof \stdClass => get_object_vars($value) !== [],
            default => true,
        };
    }

    /** The JSON Schema type of a value. */
    public static function type(mixed $value): string
    {
        return match (true) {
            $value === null => 'null',
            is_bool($value) => 'boolean',
            is_int($value) => 'integer',
            is_float($value) => 'number',
            is_string($value) => 'string',
            is_array($value) && array_is_list($value) => 'array',
            default => 'object',
        };
    }
}
