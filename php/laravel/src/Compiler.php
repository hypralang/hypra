<?php

declare(strict_types=1);

namespace Hypra\Laravel;

use Hypra\Runtime\Page;
use Symfony\Component\Process\Exception\ExceptionInterface as ProcessException;
use Symfony\Component\Process\Process;

/**
 * Compiles `.hypra` files to PHP pages with the hypra binary and caches them.
 * A cached page is recompiled when the page or any file it imports has changed.
 */
final class Compiler
{
    public function __construct(
        private readonly string $binary,
        private readonly string $cachePath,
    ) {
    }

    public function load(string $path): Page
    {
        $path = realpath($path) ?: $path;
        $compiled = $this->compiledPath($path);

        return $this->isExpired($compiled) ? $this->compile($path, $compiled) : require $compiled;
    }

    public function compiledPath(string $path): string
    {
        // a new compiler may write pages differently, so when it was built is part of the key
        $compiler = @filemtime($this->binary) ?: '';

        return $this->cachePath . '/' . sha1("{$path}|{$compiler}") . '.php';
    }

    public function isExpired(string $compiled): bool
    {
        $sources = is_file($compiled) && is_file("{$compiled}.json")
            ? json_decode((string) file_get_contents("{$compiled}.json"), true)
            : null;
        if (!is_array($sources)) {
            return true;
        }
        $compiledAt = filemtime($compiled);
        foreach ($sources as $source) {
            // modification times have one-second precision, so an edit in the same second counts as newer
            if (!is_file($source) || filemtime($source) >= $compiledAt) {
                return true;
            }
        }

        return false;
    }

    /** Removes every compiled page; returns how many files were removed. */
    public function clear(): int
    {
        $removed = 0;
        $files = [...glob($this->cachePath . '/*.php') ?: [], ...glob($this->cachePath . '/*.php.json') ?: []];
        foreach ($files as $file) {
            $removed += (int) unlink($file);
        }

        return $removed;
    }

    private function compile(string $path, string $compiled): Page
    {
        // the folder may have been removed by view:clear while the app runs, and another request may be
        // creating it right now
        if (!is_dir($this->cachePath) && !@mkdir($this->cachePath, 0755, true) && !is_dir($this->cachePath)) {
            throw new \RuntimeException("Can't create the Hypra cache folder {$this->cachePath}");
        }
        $work = $this->cachePath . '/tmp-' . bin2hex(random_bytes(8));
        $stem = $work . '/' . pathinfo($path, PATHINFO_FILENAME);
        try {
            $this->build($path, $work);
            rename("{$stem}.php", $compiled);
            if (function_exists('opcache_invalidate')) {
                opcache_invalidate($compiled, true);
            }
            // the source list comes after the page, so a crash in between leaves the cache expired
            rename("{$stem}.sources.json", "{$compiled}.json");
        } finally {
            array_map(unlink(...), glob($work . '/*') ?: []);
            @rmdir($work);
        }

        return require $compiled;
    }

    private function build(string $path, string $out): void
    {
        try {
            $process = new Process([$this->binary, 'build', $path, '--target', 'php', '--out', $out]);
            $process->run();
        } catch (ProcessException $e) {
            throw CompileError::compilerMissing($this->binary, $e->getMessage());
        }
        if ($process->getExitCode() === 127) {
            throw CompileError::compilerMissing($this->binary, 'command not found');
        }
        if (!$process->isSuccessful()) {
            throw CompileError::fromCompiler($path, $process->getErrorOutput());
        }
    }
}
