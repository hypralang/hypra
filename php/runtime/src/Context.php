<?php

declare(strict_types=1);

namespace Hypra\Runtime;

/**
 * The state of one render: output buffers, recorded actions and data access.
 * Generated pages call these methods; they are not meant to be called by hand.
 */
final class Context
{
    public string $html = '';
    public string $md = '';

    /** @var list<array{name: string, description: string, method: string, endpoint: string, params: array<string, mixed>, required: list<string>}> */
    public array $actions = [];

    /**
     * @param (\Closure(): array<string, string>)|null $formFields Hidden fields for POST forms to this site,
     *                                                     e.g. a CSRF token.
     * @param string|null $alternateUrl Where the page's Markdown is, for `<link rel="alternate">`.
     */
    public function __construct(
        private readonly array|object $data,
        private readonly bool $strict = false,
        private readonly ?\Closure $formFields = null,
        private readonly ?string $alternateUrl = null,
    ) {
    }

    public function html(string $text): void
    {
        $this->html .= $text;
    }

    public function md(string $text): void
    {
        $this->md .= $text;
    }

    /** @param mixed $action The form's URL when it comes from data; fields never go to other sites. */
    public function formFields(string $indent, mixed $action = null): void
    {
        if ($this->formFields === null || ($action !== null && !Value::isSameSite(Value::display($action)))) {
            return;
        }
        foreach (($this->formFields)() as $name => $value) {
            $this->html .= sprintf(
                "%s<input type=\"hidden\" name=\"%s\" value=\"%s\">\n",
                $indent,
                Value::escape($name),
                Value::escape($value),
            );
        }
    }

    public function htmlText(mixed $value): void
    {
        $this->html .= Value::escape(Value::display($value));
    }

    public function alternateLink(string $indent): void
    {
        if ($this->alternateUrl !== null) {
            $url = Value::escape($this->alternateUrl);
            $this->html .= "{$indent}<link rel=\"alternate\" type=\"text/markdown\" href=\"{$url}\">\n";
        }
    }

    /** A URL attribute: a URL that would run code is replaced. */
    public function htmlUrl(mixed $value): void
    {
        $this->html .= Value::escape(Value::safeUrl(Value::display($value)));
    }

    /** Text inside `<script>` or `<style>`, which can't close the element. */
    public function htmlScript(mixed $value): void
    {
        $this->html .= str_replace('</', '<\\/', Value::display($value));
    }

    /** Data inside a JSON script: a JSON string that can't close the element. */
    public function htmlScriptJson(mixed $value): void
    {
        $json = Value::json(Value::display($value));
        $this->html .= str_replace(['<', '>', '&'], ['\u003C', '\u003E', '\u0026'], $json);
    }

    /** Unescaped HTML, from `raw`. */
    public function htmlRaw(mixed $value): void
    {
        $this->html .= Value::display($value);
    }

    public function htmlJson(mixed $value): void
    {
        $this->html .= Value::escape(Value::json(Value::display($value)));
    }

    public function mdText(mixed $value): void
    {
        $this->md .= Value::markdown(Value::display($value));
    }

    public function mdUrl(mixed $value): void
    {
        $this->md .= Value::markdownUrl(Value::display($value));
    }

    /** Text inside a Markdown code span, which stays on one line. */
    public function mdCode(mixed $value): void
    {
        $this->md .= Value::markdownCode(str_replace(["\r", "\n"], ' ', Value::display($value)));
    }

    public function mdFence(mixed $value): void
    {
        $this->md .= Value::markdownCode(Value::display($value));
    }

    public function mdJson(mixed $value): void
    {
        $this->md .= Value::json(Value::display($value));
    }

    /**
     * Reads page data, e.g. ['product', 'name'] for `product.name`.
     *
     * @param non-empty-list<string> $path
     */
    public function data(array $path, string $at): mixed
    {
        $root = $path[0];
        if (!Value::has($this->data, $root)) {
            // reported once as a missing prop
            return null;
        }

        return $this->walk($root, Value::read($this->data, $root), array_slice($path, 1), $at);
    }

    /**
     * Reads a field of a loop variable or component parameter.
     *
     * @param list<string> $path
     */
    public function get(mixed $value, string $name, array $path, string $at): mixed
    {
        return $this->walk($name, $value, $path, $at);
    }

    public function each(mixed $list, string $source, string $at): iterable
    {
        // a map (an array with keys) isn't a list, as in the static build
        if (is_iterable($list) && !(is_array($list) && !array_is_list($list))) {
            return $list;
        }
        if ($list !== null) {
            $this->fail($at, "`{$source}` isn't a list");
        }

        return [];
    }

    public function concat(mixed ...$parts): string
    {
        return implode('', array_map(Value::display(...), $parts));
    }

    /**
     * @param list<array{name: string, required: bool, schema?: mixed, value?: mixed, description?: mixed}> $params
     */
    public function action(string $name, mixed $description, string $method, mixed $endpoint, array $params): void
    {
        $schemas = [];
        $required = [];
        foreach ($params as $param) {
            if (array_key_exists('value', $param)) {
                $schemas[$param['name']] = ['type' => Value::type($param['value']), 'const' => $param['value']];
            } else {
                // a field's schema, with what it's for
                $schema = $param['schema'];
                $about = Value::display($param['description'] ?? '');
                if ($about !== '') {
                    $schema['description'] = $about;
                }
                $schemas[$param['name']] = $schema;
            }
            if ($param['required']) {
                $required[] = $param['name'];
            }
        }
        foreach ($this->actions as $i => $existing) {
            if ($existing['name'] === $name) {
                $this->actions[$i]['params'] = self::mergeParams($existing['params'], $schemas);

                return;
            }
        }
        $this->actions[] = [
            'name' => $name,
            'description' => Value::display($description),
            'method' => $method,
            'endpoint' => Value::display($endpoint),
            'params' => $schemas,
            'required' => $required,
        ];
    }

    /**
     * An action repeated in a loop is one tool; values that differ between the copies become the choices of
     * that parameter, e.g. the ids of the items a remove button is shown for.
     *
     * @param array<string, mixed> $params
     * @param array<string, mixed> $more
     * @return array<string, mixed>
     */
    private static function mergeParams(array $params, array $more): array
    {
        foreach ($more as $key => $schema) {
            $old = $params[$key] ?? null;
            if (!is_array($old) || !array_key_exists('const', $schema)) {
                continue;
            }
            $value = $schema['const'];
            if (array_key_exists('const', $old)) {
                if ($old['const'] === $value) {
                    continue;
                }
                $values = [$old['const']];
            } elseif (isset($old['enum'])) {
                $values = $old['enum'];
            } else {
                continue;
            }
            if (!in_array($value, $values, true)) {
                $values[] = $value;
            }
            $params[$key] = ['type' => $old['type'], 'enum' => $values];
        }

        return $params;
    }

    /** @param list<string> $path */
    private function walk(string $name, mixed $current, array $path, string $at): mixed
    {
        $walked = $name;
        foreach ($path as $key) {
            if (!Value::has($current, $key)) {
                $this->fail($at, "`{$walked}` has no field `{$key}`");

                return null;
            }
            $current = Value::read($current, $key);
            $walked .= ".{$key}";
        }

        return $current;
    }

    private function fail(string $at, string $message): void
    {
        if ($this->strict) {
            throw new DataError("{$at}: {$message}");
        }
    }
}
