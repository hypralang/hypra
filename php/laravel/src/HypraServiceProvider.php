<?php

declare(strict_types=1);

namespace Hypra\Laravel;

use Illuminate\Contracts\Http\Kernel as HttpKernel;
use Illuminate\Foundation\Http\Kernel;
use Illuminate\Routing\Route;
use Illuminate\Routing\Router;
use Illuminate\Support\ServiceProvider;

final class HypraServiceProvider extends ServiceProvider
{
    public function register(): void
    {
        $this->mergeConfigFrom(__DIR__ . '/../config/hypra.php', 'hypra');

        $this->app->singleton(Binary::class, fn ($app) => new Binary(
            $app['config']->get('hypra.binary'),
            $app->storagePath('hypra'),
        ));
        $this->app->singleton(Compiler::class, fn ($app) => new Compiler(
            $app->make(Binary::class)->path(),
            $app['config']->get('hypra.compiled') ?? $app['config']->get('view.compiled') . '/hypra',
        ));
    }

    public function boot(): void
    {
        $this->publishes([__DIR__ . '/../config/hypra.php' => $this->app->configPath('hypra.php')], 'hypra-config');

        $this->app['view']->addExtension('hypra', 'hypra', fn () => new HypraEngine(
            $this->app->make(Compiler::class),
            $this->app,
        ));

        if ($this->app['config']->get('hypra.negotiate')) {
            // The HTTP kernel owns the middleware groups and syncs them onto the router, so add it there.
            $kernel = $this->app->make(HttpKernel::class);
            if ($kernel instanceof Kernel) {
                $kernel->appendMiddlewareToGroup('web', NegotiateFormat::class);
            } else {
                $this->app['router']->pushMiddlewareToGroup('web', NegotiateFormat::class);
            }
        }

        if ($this->app->runningInConsole()) {
            $this->commands([ClearCommand::class, CompileCommand::class, InstallCommand::class]);
            // `php artisan optimize` compiles the pages too, and `optimize:clear` removes them (Laravel 11.27+)
            if (method_exists($this, 'optimizes')) {
                $this->optimizes(optimize: 'hypra:compile', clear: 'hypra:clear', key: 'hypra');
            }
        }

        /*
         * Connects a page's `action("cart.add", ...)` to a handler:
         *     Route::hypra('cart.add', [CartController::class, 'add']);
         */
        Router::macro('hypra', function (string $action, mixed $handler): Route {
            /** @var Router $this */
            return $this->post('/_a/' . $action, $handler)->name('hypra.' . $action);
        });
    }
}
