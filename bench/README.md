# LLM benchmark

Measures how well a language model writes Hypra from [`docs/spec.md`](../docs/spec.md) alone. Every change to the
language or the spec can be checked against it: a change that makes Hypra harder for models shows up here.

## Tasks

[`tasks/`](tasks) holds 20 tasks, from a static page to forms, components, layouts, Alpine.js and htmx.
Each task is a folder with:

| File | Contents |
| --- | --- |
| `task.md` | what to build, in plain language |
| `data.json` | the data the page receives |
| `expect.txt` | strings the HTML must contain, one per line (`a\|\|b` accepts either) |

[`reference/`](reference) has a solution for every task, so the tasks are known to be solvable.

## Running a model

Give the model the spec and one task at a time. Save its answer as `runs/<model>/<task>.1.hypra` before it sees
any compiler output. To measure how well it recovers from errors, let it run `hypra check` and save each fix as
`<task>.2.hypra`, `<task>.3.hypra`.

Then score the run:

```console
$ cargo build --release
$ python3 bench/score.py bench/runs/<model>
```

A solution passes when it compiles without errors and its HTML contains everything in `expect.txt`.
The report shows the pass rate on the first try and with compiler feedback, plus the most common errors.

## Results

| Model | Spec | First try | First try without warnings | With compiler feedback |
| --- | --- | --- | --- | --- |
| Claude Haiku 5.5 | with "Writing good pages" | 19/20 | 18/20 | 20/20 |
| Claude Haiku 5.5 | before "Writing good pages" | 20/20 | 5/20 | 20/20 |

The solutions of the current run are in [`runs/haiku`](runs/haiku). The second row is an earlier run, before the
spec explained how to write good pages; its warnings were counted with today's checks.

Every first try compiles or is fixed after one round of compiler feedback, so the benchmark mainly guards
against changes that make Hypra harder for models.
