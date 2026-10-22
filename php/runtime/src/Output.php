<?php

declare(strict_types=1);

namespace Hypra\Runtime;

/**
 * Everything one page renders to: HTML for browsers, Markdown for LLMs,
 * JSON for apps and WebMCP tool definitions for AI agents.
 */
final class Output
{
    /**
     * @param array<string, mixed> $json
     * @param list<array<string, mixed>> $webmcp
     */
    public function __construct(
        public readonly string $html,
        public readonly string $markdown,
        public readonly array $json,
        public readonly array $webmcp,
    ) {
    }

    /** @param array{title: mixed, description: mixed, lang: mixed, canonical: mixed} $meta */
    public static function assemble(Context $c, array $meta): self
    {
        $markdown = rtrim($c->md) . "\n";
        if ($c->actions !== []) {
            $markdown .= "\n## Actions\n\n";
            foreach ($c->actions as $a) {
                $markdown .= sprintf(
                    "- %s: %s → %s %s\n",
                    Value::markdownSpan("{$a['name']}(" . self::signature($a) . ')'),
                    Value::markdown($a['description']),
                    $a['method'],
                    Value::markdown($a['endpoint']),
                );
            }
        }

        $optional = static fn (mixed $value): ?string => $value === null ? null : Value::display($value);
        $actions = [];
        $tools = [];
        foreach ($c->actions as $a) {
            // an empty parameter list is still a JSON object
            $properties = $a['params'] === [] ? new \stdClass() : $a['params'];
            $actions[] = [
                'name' => $a['name'],
                'description' => $a['description'],
                'method' => $a['method'],
                'endpoint' => $a['endpoint'],
                'params' => $properties,
                'required' => $a['required'],
            ];
            $tools[] = [
                'name' => $a['name'],
                'description' => $a['description'],
                'inputSchema' => ['type' => 'object', 'properties' => $properties, 'required' => $a['required']],
            ];
        }

        return new self(
            $c->html,
            $markdown,
            [
                'title' => Value::display($meta['title']),
                'description' => $optional($meta['description']),
                'lang' => Value::display($meta['lang']),
                'canonical' => $optional($meta['canonical']),
                'actions' => $actions,
            ],
            $tools,
        );
    }

    /** Compact parameter list for the Markdown action index, e.g. `stars: integer 1..5, body?: string ..2000`. */
    private static function signature(array $action): string
    {
        $parts = [];
        foreach ($action['params'] as $name => $schema) {
            if (array_key_exists('const', $schema)) {
                $parts[] = "{$name}: " . Value::json($schema['const']);
                continue;
            }
            $type = $schema['format'] ?? $schema['type'] ?? 'any';
            if (isset($schema['enum'])) {
                $type = implode('|', array_map(Value::display(...), $schema['enum']));
            }
            $low = $schema['minimum'] ?? $schema['minLength'] ?? null;
            $high = $schema['maximum'] ?? $schema['maxLength'] ?? null;
            if ($low !== null || $high !== null) {
                $type .= ' ' . Value::display($low) . '..' . Value::display($high);
            }
            $optional = in_array($name, $action['required'], true) ? '' : '?';
            $parts[] = "{$name}{$optional}: {$type}";
        }

        return implode(', ', $parts);
    }
}
