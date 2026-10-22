<?php

declare(strict_types=1);

namespace Hypra\Laravel;

use Illuminate\Console\Command;
use Illuminate\Support\Facades\Http;
use PharData;
use Symfony\Component\Process\Process;

/**
 * Downloads the compiler for this machine from the GitHub release of this package's version, checks it against
 * the release's SHA-256 checksums, and puts it where the package finds it. No Rust needed.
 */
final class InstallCommand extends Command
{
    protected $signature = 'hypra:install {--release= : the compiler version, e.g. 0.1.0; this package\'s version by default} {--force : download even if it\'s installed}';

    protected $description = 'Download the hypra compiler for this machine';

    public function handle(Binary $binary): int
    {
        $version = $this->option('release') ?: Binary::packageVersion();
        if ($version === null) {
            $this->components->error('This is a development copy of hypra/laravel, so there\'s no release to match. Pass --release=0.1.0, or set HYPRA_BINARY.');

            return self::FAILURE;
        }
        $target = Binary::target();
        if ($target === null) {
            $this->components->error('There\'s no prebuilt compiler for ' . PHP_OS_FAMILY . ' on ' . php_uname('m') . '. Build it with `cargo install hypra` and set HYPRA_BINARY.');

            return self::FAILURE;
        }

        $path = $binary->installed();
        if (!$this->option('force') && $this->installedVersion($path) === $version) {
            $this->components->info("hypra {$version} is already installed.");

            return self::SUCCESS;
        }

        $base = rtrim((string) config('hypra.releases'), '/') . "/v{$version}";
        $archive = "hypra-{$version}-{$target}.tar.gz";
        $sums = Http::get("{$base}/SHA256SUMS");
        $file = Http::get("{$base}/{$archive}");
        if ($sums->failed() || $file->failed()) {
            $this->components->error("Can't download {$base}/{$archive}.");

            return self::FAILURE;
        }
        $expected = $this->checksum($sums->body(), $archive);
        if ($expected === null || !hash_equals($expected, hash('sha256', $file->body()))) {
            $this->components->error("{$archive} doesn't match its SHA-256 checksum, so it wasn't installed.");

            return self::FAILURE;
        }

        $this->extract($file->body(), "hypra-{$version}-{$target}", $path);
        if ($this->installedVersion($path) !== $version) {
            @unlink($path);
            $this->components->error("The downloaded compiler doesn't run on this machine.");

            return self::FAILURE;
        }
        $this->components->info("Installed hypra {$version} ({$target}) to {$path}.");

        return self::SUCCESS;
    }

    /** The checksum of one file in a SHA256SUMS listing: `<hash>  <file>` per line, or `<hash> *<file>`. */
    private function checksum(string $sums, string $file): ?string
    {
        foreach (explode("\n", $sums) as $line) {
            [$hash, $name] = explode(' ', trim($line), 2) + [1 => ''];
            if (ltrim($name, ' *') === $file) {
                return strtolower($hash);
            }
        }

        return null;
    }

    private function extract(string $tarGz, string $folder, string $path): void
    {
        $work = sys_get_temp_dir() . '/hypra-install-' . bin2hex(random_bytes(6));
        mkdir($work, 0700, true);
        try {
            file_put_contents("{$work}/archive.tar.gz", $tarGz);
            (new PharData("{$work}/archive.tar.gz"))->extractTo($work);
            if (!is_dir(dirname($path))) {
                mkdir(dirname($path), 0755, true);
            }
            copy("{$work}/{$folder}/" . basename($path), $path);
            chmod($path, 0755);
        } finally {
            (new \Illuminate\Filesystem\Filesystem())->deleteDirectory($work);
        }
    }

    /** The version an installed compiler reports, or null if there's none or it doesn't run. */
    private function installedVersion(string $path): ?string
    {
        if (!is_file($path)) {
            return null;
        }
        $process = new Process([$path, '--version']);
        $process->run();

        $output = trim($process->getOutput());

        return $process->isSuccessful() && str_starts_with($output, 'hypra ') ? substr($output, 6) : null;
    }
}
