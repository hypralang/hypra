<?php

declare(strict_types=1);

namespace Hypra\Laravel;

use Closure;
use Hypra\Runtime\Output;
use Hypra\Runtime\Value;
use Illuminate\Http\Request;
use Illuminate\Http\Response as IlluminateResponse;
use Illuminate\View\View;
use Symfony\Component\HttpFoundation\Response;

/**
 * Lets one URL serve a page as HTML, Markdown or JSON depending on the Accept header.
 * Only a response made from a `.hypra` view changes; other views rendered during the request, such as an
 * email, don't affect it, and neither does middleware that edits the HTML.
 */
final class NegotiateFormat
{
    /** Request attribute with the Output of every page rendered during the request, by view path. */
    public const RENDERED = 'hypra.rendered';

    public function handle(Request $request, Closure $next): Response
    {
        /** @var Response $response */
        $response = $next($request);

        $page = $this->renderedPage($request, $response);
        if ($page === null) {
            return $response;
        }

        $format = Format::fromRequest($request);
        $response->setVary('Accept', false);
        $response->headers->set('Content-Type', $format->contentType());
        $response->headers->set('X-Content-Type-Options', 'nosniff');
        match ($format) {
            Format::Html => null,
            Format::Markdown => $response->setContent($page->markdown),
            Format::Json => $response->setContent(Value::json($page->json)),
        };

        return $response;
    }

    /**
     * The page this response was made from: the `.hypra` view it was made from, or else the page whose HTML it is.
     * Middleware around the whole app, like a debug toolbar, edits the HTML only after this has answered.
     */
    private function renderedPage(Request $request, Response $response): ?Output
    {
        /** @var array<string, Output> $rendered */
        $rendered = $request->attributes->get(self::RENDERED, []);
        $view = $response instanceof IlluminateResponse ? $response->getOriginalContent() : null;
        if ($view instanceof View) {
            return $rendered[$view->getPath()] ?? null;
        }
        $html = $response->getContent();
        foreach ($rendered as $output) {
            if ($output->html === $html) {
                return $output;
            }
        }

        return null;
    }
}
