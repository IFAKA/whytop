# whytop demo recording

The walkthrough is deterministic and does not contact a model server. It uses
the `WHYTOP_DEMO=1` process fixtures and local streaming demo engine.

Install the official terminal recording tools first:

```sh
brew install asciinema agg
```

From the repository root, regenerate both artifacts with:

```sh
python3 demo/record-demo.py && agg demo/whytop.cast demo/whytop.gif
```

Replay the selectable recording with `asciinema play demo/whytop.cast`. The
driver fixes the terminal at 120×36, sends controls on a schedule, and exits
with an error if the application does not finish. The GIF is intended as a
small README preview; the cast preserves selectable text and timing.
