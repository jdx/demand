# Demo recordings

From the repository root, run an example with `cargo run --example grid_select`.
Each GIF is generated from the corresponding VHS tape and a real example binary.

## Record with Docker

With Docker running, mise builds the recording image and compiles all examples
before recording:

```sh
mise run recording assets/grid-select.tape
mise run "recordings input"
mise run "recordings themes"
```

The recording container uses a named `demand-vhs-target` volume for build output,
so it does not overwrite native binaries in your local `target` directory.

## Record locally

Install VHS 0.11.0, ttyd, and FFmpeg, then run:

```sh
bash assets/record.sh --local assets/grid-select.tape
# All prompt and theme recordings:
bash assets/record.sh --local
```

VHS also needs a Chromium browser; it downloads one if necessary.
The script builds examples before starting VHS, stops on the first failure, and
replaces each GIF only after a new recording is produced. Output GIFs use the
same path and basename as their tapes.

## Add or update a demo

1. Add a runnable example in `examples/`.
2. Add a tape in `assets/` that sources `assets/vhs/common.tape`, requires the
   example binary, and waits for the prompt before sending keys.
3. Show the feature and its submitted result, with pauses long enough to read.
4. Record the tape, inspect the GIF for clipping and missed input, and link it
   from the corresponding README section.

Shared terminal styling lives in `assets/vhs/common.tape`. Keep setup there so
recordings have consistent framing and readable text. Theme demos use the same
terminal background to make prompt color differences easy to compare.
