<?php

declare(strict_types=1);

namespace Hypra\Laravel;

use Illuminate\Contracts\Container\Container;
use Illuminate\Contracts\View\Engine;
use Illuminate\Http\Request;

/**
 * Renders `.hypra` views as HTML. The other formats of a page are kept on the request, so
 * NegotiateFormat can answer with Markdown or JSON when the page is the response.
 */
final class HypraEngine implements Engine
{
    public function __construct(
        private readonly Compiler $compiler,
        private readonly Container $app,
    ) {
    }

    /** @param array<string, mixed> $data */
    public function get($path, array $data = []): string
    {
        /** @var Request $request */
        $request = $this->app->make('request');
        $config = $this->app->make('config');

        $output = $this->compiler->load($path)->render(
            $data,
            strict: (bool) $config->get('hypra.strict'),
            formFields: fn (): array => $request->hasSession() ? ['_token' => $request->session()->token()] : [],
            // a path, not a full URL: the host comes from the request's Host header, which visitors choose
            alternateUrl: $config->get('hypra.negotiate') ? $request->getRequestUri() : null,
        );

        $rendered = $request->attributes->get(NegotiateFormat::RENDERED, []);
        $rendered[$path] = $output;
        $request->attributes->set(NegotiateFormat::RENDERED, $rendered);

        return $output->html;
    }
}
