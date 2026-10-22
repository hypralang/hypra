<?php

declare(strict_types=1);

namespace Hypra\Laravel\Tests;

use Hypra\Laravel\CompileError;
use Hypra\Laravel\HypraServiceProvider;
use Illuminate\Contracts\Http\Kernel;
use Illuminate\Database\Eloquent\Model;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\Artisan;
use Illuminate\Routing\Router;
use Orchestra\Testbench\TestCase;

/**
 * Renders the repository's example pages through Laravel. Needs a built compiler:
 * `cargo build` in the repository root, or HYPRA_BINARY pointing at one.
 */
final class HypraTest extends TestCase
{
    private const REPO = __DIR__ . '/../../..';

    private string $work;

    protected function setUp(): void
    {
        $this->work = sys_get_temp_dir() . '/hypra-laravel-' . bin2hex(random_bytes(6));
        mkdir("{$this->work}/compiled", 0755, true);
        $this->copy(self::REPO . '/examples', "{$this->work}/views");
        $description = 'A test page with a description long enough for search results.';
        file_put_contents("{$this->work}/views/profile.hypra", <<<HYPRA
            page(title: "Profile", description: "{$description}") {
              props(user: User)
              h1 "{user.name}"
              if user.avatar { img(user.avatar, alt: "Avatar of {user.name}") } else { p "No avatar yet." }
              form("/local", name: "local") { button "Save" }
              form("https://newsletter.example.com/subscribe", name: "newsletter") { button "Subscribe" }
            }
            HYPRA);

        parent::setUp();
    }

    protected function tearDown(): void
    {
        parent::tearDown();
        $this->remove($this->work);
    }

    protected function getPackageProviders($app): array
    {
        return [HypraServiceProvider::class];
    }

    protected function defineEnvironment($app): void
    {
        $app['config']->set('app.key', 'base64:' . base64_encode(str_repeat('k', 32)));
        $app['config']->set('hypra.binary', getenv('HYPRA_BINARY') ?: self::REPO . '/target/debug/hypra');
        $app['config']->set('view.compiled', "{$this->work}/compiled");
        $app['config']->set('hypra.strict', true);
        $app['config']->set('view.paths', ["{$this->work}/views"]);
    }

    protected function defineRoutes($router): void
    {
        $product = json_decode((string) file_get_contents(self::REPO . '/examples/product.json'), true)['product'];

        $router->middleware('web')->group(function (Router $router) use ($product): void {
            $router->get('/shoes/{id}', fn (int $id) => view('product', ['product' => [...$product, 'id' => $id]]));
            $router->hypra('cart.add', fn (Request $request) => "added {$request->input('id')}");
            $router->get('/profile', function () {
                $user = new class extends Model {
                    protected $guarded = [];
                };

                return view('profile', ['user' => $user->forceFill(['name' => 'Ana', 'avatar' => null])]);
            });
            // renders a page (as an email would) but answers with something else
            $router->get('/report', function () use ($product) {
                view('product', ['product' => [...$product, 'id' => 1]])->render();

                return response('plain text', 200, ['Content-Type' => 'text/plain']);
            });
        });
    }

    public function test_renders_html_with_a_csrf_token_in_every_post_form(): void
    {
        $response = $this->get('/shoes/7', ['Accept' => 'text/html,*/*;q=0.8']);

        $response->assertOk();
        $response->assertHeader('Content-Type', 'text/html; charset=UTF-8');
        $response->assertHeader('Vary', 'Accept');
        $response->assertSee('<title>Runner X – Running Shoes | Shop</title>', false);
        $response->assertSee('<link rel="canonical" href="https://shop.test/shoes/7">', false);
        $token = '<input type="hidden" name="_token" value="' . session()->token() . '">';
        $this->assertSame(2, substr_count($response->getContent(), $token));
    }

    public function test_null_eloquent_attributes_exist_in_strict_mode(): void
    {
        $this->get('/profile')->assertOk()->assertSee('<p>No avatar yet.</p>', false);
    }

    public function test_csrf_tokens_only_go_to_this_site(): void
    {
        $html = $this->get('/profile')->getContent();

        $this->assertSame(1, substr_count($html, 'name="_token"'));
        $this->assertStringContainsString('tooldescription="local">' . "\n" . '    <input type="hidden" name="_token"', $html);
    }

    public function test_links_the_markdown_version_at_the_same_url(): void
    {
        // a path with its query, never the visitor's Host header
        $this->get('/shoes/7?size=42', ['Host' => 'evil.example'])
            ->assertSee('<link rel="alternate" type="text/markdown" href="/shoes/7?size=42">', false);
    }

    public function test_pages_rendered_for_something_else_leave_the_response_alone(): void
    {
        $response = $this->get('/report', ['Accept' => 'text/markdown']);

        $response->assertOk()->assertHeader('Content-Type', 'text/plain; charset=UTF-8');
        $this->assertSame('plain text', $response->getContent());
        $this->assertFalse($response->headers->has('Vary'));
    }

    public function test_view_clear_removes_compiled_pages(): void
    {
        $this->get('/shoes/7')->assertOk();
        $this->assertNotEmpty(glob("{$this->work}/compiled/hypra/*.php"));

        Artisan::call('view:clear');

        $this->assertEmpty(glob("{$this->work}/compiled/hypra/*.php"));
        $this->get('/shoes/7')->assertOk();
    }

    public function test_serves_markdown_to_clients_that_ask_for_it(): void
    {
        $response = $this->get('/shoes/7', ['Accept' => 'text/markdown']);

        $response->assertOk();
        $response->assertHeader('Content-Type', 'text/markdown; charset=UTF-8');
        $response->assertHeader('X-Content-Type-Options', 'nosniff');
        $this->assertStringContainsString("# Runner X\n", $response->getContent());
        $this->assertStringContainsString('- `cart.add(id: 7)`: Add to cart → POST /_a/cart.add', $response->getContent());
    }

    public function test_negotiates_alongside_middleware_that_edits_html(): void
    {
        $this->app->make(Kernel::class)->pushMiddleware(EditsHtml::class);
        $this->get('/shoes/7')->assertSee('<!-- edited -->', false);
        $response = $this->get('/shoes/7', ['Accept' => 'text/markdown']);

        $response->assertHeader('Content-Type', 'text/markdown; charset=UTF-8');
        $this->assertStringStartsWith("---\ntitle:", (string) $response->getContent());
    }

    public function test_serves_json_to_clients_that_ask_for_it(): void
    {
        $response = $this->get('/shoes/7', ['Accept' => 'application/json']);

        $response->assertOk();
        $response->assertHeader('Content-Type', 'application/json');
        $response->assertJsonPath('title', 'Runner X – Running Shoes');
        $response->assertJsonPath('actions.0.params.id.const', 7);
    }

    public function test_route_macro_connects_an_action_to_its_handler(): void
    {
        $this->post('/_a/cart.add', ['id' => 7])->assertOk()->assertSeeText('added 7');
        $this->assertSame('/_a/cart.add', route('hypra.cart.add', absolute: false));
    }

    public function test_recompiles_when_an_imported_file_changes(): void
    {
        $this->get('/shoes/7')->assertSee('| Shop</title>', false);

        $layout = "{$this->work}/views/layouts/shop.hypra";
        file_put_contents($layout, str_replace('" | Shop"', '" | Shoe Shop"', (string) file_get_contents($layout)));

        $this->get('/shoes/7')->assertSee('| Shoe Shop</title>', false);
    }

    public function test_a_new_compiler_compiles_pages_again(): void
    {
        $binary = "{$this->work}/hypra";
        copy((string) $this->app['config']->get('hypra.binary'), $binary);
        chmod($binary, 0755);
        $this->app['config']->set('hypra.binary', $binary);
        $this->app->forgetInstance(\Hypra\Laravel\Compiler::class);
        $compiler = $this->app->make(\Hypra\Laravel\Compiler::class);
        $before = $compiler->compiledPath("{$this->work}/views/product.hypra");

        touch($binary, time() + 60);
        clearstatcache();

        $this->assertNotSame($before, $compiler->compiledPath("{$this->work}/views/product.hypra"));
    }

    public function test_compile_errors_point_at_the_hypra_source(): void
    {
        $card = "{$this->work}/views/components/review-card.hypra";
        file_put_contents($card, str_replace('p "{review.body}"', 'img("x.jpg")', (string) file_get_contents($card)));

        $this->withoutExceptionHandling();
        try {
            $this->get('/shoes/7');
            $this->fail('expected a compile error');
        } catch (\Throwable $e) {
            $error = $e instanceof CompileError ? $e : $e->getPrevious();
            $this->assertInstanceOf(CompileError::class, $error);
            $this->assertStringContainsString('error[A11Y-001]: image has no alt text', $error->getMessage());
            $this->assertStringContainsString('components/review-card.hypra:4:9', $error->getMessage());
        }
    }

    private function copy(string $from, string $to): void
    {
        mkdir($to, 0755, true);
        foreach (scandir($from) as $name) {
            if ($name === '.' || $name === '..') {
                continue;
            }
            is_dir("{$from}/{$name}") ? $this->copy("{$from}/{$name}", "{$to}/{$name}") : copy("{$from}/{$name}", "{$to}/{$name}");
        }
    }

    private function remove(string $path): void
    {
        if (is_dir($path)) {
            foreach (scandir($path) as $name) {
                if ($name !== '.' && $name !== '..') {
                    $this->remove("{$path}/{$name}");
                }
            }
            rmdir($path);
        } elseif (file_exists($path)) {
            unlink($path);
        }
    }
}

/** Adds a comment to HTML responses, like a debug toolbar. */
final class EditsHtml
{
    public function handle(Request $request, \Closure $next): \Symfony\Component\HttpFoundation\Response
    {
        $response = $next($request);
        if (str_starts_with((string) $response->headers->get('Content-Type'), 'text/html')) {
            $response->setContent($response->getContent() . '<!-- edited -->');
        }

        return $response;
    }
}
