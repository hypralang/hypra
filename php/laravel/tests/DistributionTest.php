<?php

declare(strict_types=1);

namespace Hypra\Laravel\Tests;

use Hypra\Laravel\Binary;
use Hypra\Laravel\HypraServiceProvider;
use Illuminate\Support\Facades\Http;
use Orchestra\Testbench\TestCase;
use PharData;

/** Installing the compiler without Rust, and compiling every page ahead of time. */
final class DistributionTest extends TestCase
{
    private const REPO = __DIR__ . '/../../..';

    private string $work;

    protected function setUp(): void
    {
        $this->work = sys_get_temp_dir() . '/hypra-dist-' . bin2hex(random_bytes(6));
        mkdir("{$this->work}/views/components", 0755, true);
        $description = 'A test page with a description long enough for search results.';
        file_put_contents("{$this->work}/views/home.hypra", "use \"components/card.hypra\"\n\npage(title: \"Home\", description: \"{$description}\") {\n  h1 \"Home\"\n  Card(title: \"x\")\n}\n");
        file_put_contents("{$this->work}/views/components/card.hypra", "component Card(title: Text) { h2 \"{title}\" }\n");
        parent::setUp();
    }

    protected function tearDown(): void
    {
        parent::tearDown();
        (new \Illuminate\Filesystem\Filesystem())->deleteDirectory($this->work);
    }

    protected function getPackageProviders($app): array
    {
        return [HypraServiceProvider::class];
    }

    protected function defineEnvironment($app): void
    {
        $app->useStoragePath("{$this->work}/storage");
        $app['config']->set('hypra.binary', getenv('HYPRA_BINARY') ?: self::REPO . '/target/debug/hypra');
        $app['config']->set('hypra.releases', 'https://releases.test');
        $app['config']->set('view.compiled', "{$this->work}/compiled");
        $app['config']->set('view.paths', ["{$this->work}/views"]);
    }

    /** A release archive whose "compiler" is a script that reports the given version. */
    private function release(string $version): array
    {
        $target = Binary::target();
        $folder = "{$this->work}/build/hypra-{$version}-{$target}";
        mkdir($folder, 0755, true);
        file_put_contents("{$folder}/hypra", "#!/bin/sh\necho \"hypra {$version}\"\n");
        $tar = "{$this->work}/build/hypra-{$version}-{$target}.tar";
        $phar = new PharData($tar);
        $phar->buildFromDirectory("{$this->work}/build", '/hypra-/');
        $phar->compress(\Phar::GZ);
        $archive = (string) file_get_contents("{$tar}.gz");

        return ["hypra-{$version}-{$target}.tar.gz", $archive];
    }

    public function test_install_downloads_and_checks_the_compiler(): void
    {
        [$name, $archive] = $this->release('9.9.9');
        Http::fake([
            'releases.test/v9.9.9/SHA256SUMS' => Http::response(hash('sha256', $archive) . " *{$name}\nabc  other.tar.gz\n"),
            "releases.test/v9.9.9/{$name}" => Http::response($archive),
        ]);

        $this->artisan('hypra:install', ['--release' => '9.9.9'])->assertSuccessful();

        $installed = $this->app->make(Binary::class)->installed();
        $this->assertFileExists($installed);
        $this->assertTrue(is_executable($installed));
        $this->artisan('hypra:install', ['--release' => '9.9.9'])->expectsOutputToContain('already installed')->assertSuccessful();
    }

    public function test_install_refuses_a_download_that_doesnt_match_its_checksum(): void
    {
        [$name, $archive] = $this->release('9.9.9');
        Http::fake([
            'releases.test/v9.9.9/SHA256SUMS' => Http::response(hash('sha256', 'something else') . "  {$name}\n"),
            "releases.test/v9.9.9/{$name}" => Http::response($archive),
        ]);

        $this->artisan('hypra:install', ['--release' => '9.9.9'])->assertFailed();
        $this->assertFileDoesNotExist($this->app->make(Binary::class)->installed());
    }

    public function test_the_installed_compiler_is_used_unless_one_is_configured(): void
    {
        $binary = new Binary(null, "{$this->work}/bin");
        $this->assertSame('hypra', $binary->path());
        mkdir("{$this->work}/bin");
        touch($binary->installed());
        $this->assertSame($binary->installed(), $binary->path());
        $this->assertSame('/opt/hypra', (new Binary('/opt/hypra', "{$this->work}/bin"))->path());
    }

    public function test_targets_name_the_release_builds(): void
    {
        $this->assertSame('x86_64-unknown-linux-musl', Binary::target('Linux', 'x86_64'));
        $this->assertSame('aarch64-apple-darwin', Binary::target('Darwin', 'arm64'));
        $this->assertSame('x86_64-pc-windows-msvc', Binary::target('Windows', 'AMD64'));
        $this->assertNull(Binary::target('Linux', 'riscv64'));
    }

    public function test_compile_compiles_every_page_and_skips_components(): void
    {
        $this->artisan('hypra:compile')->expectsOutputToContain('Compiled 1 Hypra page.')->assertSuccessful();

        file_put_contents("{$this->work}/views/broken.hypra", "page(title: \"Broken\") {\n  h1 \"x\"\n}\n");
        $this->artisan('hypra:compile')->expectsOutputToContain('1 of 2 Hypra pages doesn\'t compile')->assertFailed();
    }
}
