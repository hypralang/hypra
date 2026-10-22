<?php

declare(strict_types=1);

namespace Hypra\Laravel;

use Illuminate\Console\Command;

final class ClearCommand extends Command
{
    protected $signature = 'hypra:clear';

    protected $description = 'Remove all compiled Hypra pages (view:clear removes them too)';

    public function handle(Compiler $compiler): int
    {
        $removed = $compiler->clear();
        $this->components->info("Compiled Hypra pages cleared ({$removed} files).");

        return self::SUCCESS;
    }
}
