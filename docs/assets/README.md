# Assets Directory

This directory contains assets for the documentation.

## flux-demo.gif

To create a demo GIF for the README:

1. Install a terminal recorder like `asciinema` and `agg`:
   ```bash
   # Install asciinema
   pip install asciinema
   
   # Install agg (asciinema gif generator)
   cargo install --git https://github.com/asciinema/agg
   ```

2. Record a demo session:
   ```bash
   asciinema rec demo.cast
   
   # Run some flux commands
   flux pack ./test-data -o test.tar.zst --progress
   flux extract test.tar.zst -o ./extracted --progress
   flux inspect test.tar.zst
   
   # Exit recording with Ctrl+D
   ```

3. Convert to GIF:
   ```bash
   agg demo.cast flux-demo.gif
   ```

4. Optimize the GIF (optional):
   ```bash
   gifsicle -O3 flux-demo.gif -o flux-demo-optimized.gif
   ```

The GIF should showcase:
- Progress bars during packing
- Smart compression in action
- Fast extraction with progress
- Archive inspection