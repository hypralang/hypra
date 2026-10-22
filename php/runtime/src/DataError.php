<?php

declare(strict_types=1);

namespace Hypra\Runtime;

/** Thrown in strict mode when the data doesn't match what a page reads, e.g. a missing field. */
final class DataError extends \RuntimeException
{
}
