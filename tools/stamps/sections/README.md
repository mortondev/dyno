# Stamp recipes by world section

One file per world section (`hearthstead.py`, `birchrise.py`, ...), loaded by
`tools/stamps/build.py`. A file starts with:

    from build import recipe
    from kit import *

and names its stamps with the section's prefix (`birchrise_woodpile`), so two
sections never define the same stamp. Shared pieces stay in `kit.py` and the
main `build.py`; change those only through the section brief's global changes.
