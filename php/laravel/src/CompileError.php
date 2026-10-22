<?php

declare(strict_types=1);

namespace Hypra\Laravel;

/** A page that the hypra compiler rejected, with the compiler's error messages. */
final class CompileError extends \RuntimeException
{
    public static function fromCompiler(string $path, string $output): self
    {
        return new self("Hypra could not compile {$path}:\n\n" . trim($output));
    }

    public static function compilerMissing(string $binary, string $reason): self
    {
        return new self(
            "The hypra compiler could not be run as `{$binary}`: {$reason}\n"
            . 'Install it or point HYPRA_BINARY at it.',
        );
    }
}
