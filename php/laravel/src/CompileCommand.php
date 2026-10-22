<?php

declare(strict_types=1);

namespace Hypra\Laravel;

use Illuminate\Console\Command;
use Illuminate\View\Factory;
use Symfony\Component\Finder\Finder;

/** Compiles every `.hypra` page ahead of time, e.g. when deploying, so no request has to. */
final class CompileCommand extends Command
{
    protected $signature = 'hypra:compile';

    protected $description = 'Compile all Hypra pages (php artisan optimize runs this too)';

    public function handle(Compiler $compiler, Factory $views): int
    {
        $finder = $views->getFinder();
        $folders = [...$finder->getPaths(), ...array_merge([], ...array_values($finder->getHints()))];
        $folders = array_filter(array_unique($folders), 'is_dir');
        $compiled = 0;
        $failed = 0;
        foreach ($folders === [] ? [] : Finder::create()->files()->name('*.hypra')->in($folders) as $file) {
            // components and layouts are compiled with the pages that use them
            if (preg_match('/^\s*page\s*\(/m', $file->getContents()) !== 1) {
                continue;
            }
            try {
                $compiler->load($file->getPathname());
                $compiled++;
            } catch (CompileError $e) {
                $this->components->error($e->getMessage());
                $failed++;
            }
        }
        if ($failed > 0) {
            $this->components->error("{$failed} of " . ($compiled + $failed) . ' Hypra pages don\'t compile.');

            return self::FAILURE;
        }
        $this->components->info("Compiled {$compiled} Hypra pages.");

        return self::SUCCESS;
    }
}
