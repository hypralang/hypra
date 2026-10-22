<?php

declare(strict_types=1);

namespace Hypra\Laravel;

use Composer\InstalledVersions;

/**
 * Where the hypra compiler is: the `hypra.binary` setting, else the copy `php artisan hypra:install` downloaded,
 * else `hypra` on the PATH.
 */
final class Binary
{
    public function __construct(
        private readonly ?string $configured,
        private readonly string $installDir,
    ) {
    }

    public function path(): string
    {
        if ($this->configured !== null && $this->configured !== '') {
            return $this->configured;
        }

        return is_file($this->installed()) ? $this->installed() : 'hypra';
    }

    /** Where `hypra:install` puts the compiler. */
    public function installed(): string
    {
        return $this->installDir . '/' . (PHP_OS_FAMILY === 'Windows' ? 'hypra.exe' : 'hypra');
    }

    /** The version of this package, which is the compiler version it's made for; null for a development copy. */
    public static function packageVersion(): ?string
    {
        if (!class_exists(InstalledVersions::class) || !InstalledVersions::isInstalled('hypra/laravel')) {
            return null;
        }
        $version = ltrim((string) InstalledVersions::getPrettyVersion('hypra/laravel'), 'v');

        return preg_match('/^\d+\.\d+\.\d+$/', $version) === 1 ? $version : null;
    }

    /** The release build for this machine, e.g. `x86_64-unknown-linux-musl`; null if there's none. */
    public static function target(string $os = PHP_OS_FAMILY, ?string $machine = null): ?string
    {
        $arch = match (strtolower($machine ?? php_uname('m'))) {
            'x86_64', 'amd64' => 'x86_64',
            'aarch64', 'arm64' => 'aarch64',
            default => null,
        };
        $system = match ($os) {
            'Linux' => 'unknown-linux-musl',
            'Darwin' => 'apple-darwin',
            'Windows' => 'pc-windows-msvc',
            default => null,
        };

        return $arch === null || $system === null ? null : "{$arch}-{$system}";
    }
}
