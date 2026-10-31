from libspec import Constraint


class PackagingTomlsValid(Constraint):
    """CONSTRAINT-ID: CON-WD01

    Every `packaging/*.toml` parses, has no `scripts` or `maintainer` key
    (the maintainer comes from the DEB_MAINTAINER repository variable), and
    every `files` entry has a `src` that is the word binary (or asset) or a
    path, relative to the toml file as mkdeb reads it, that exists, and a
    `dst` under usr/.
    ENFORCEMENT (reconcile.py): packaging.valid is true.
    """
    constraint_id = "CON-WD01"
    enforcement_logic = "{{ packaging.valid }}"


class ExampleConfigCompiledIn(Constraint):
    """CONSTRAINT-ID: CON-WD02

    The example config is compiled into the binary (WMDEB-002):
    `include_str!` of config.example.toml appears in src/.
    ENFORCEMENT (reconcile.py): example_config.compiled_in is true.
    """
    constraint_id = "CON-WD02"
    enforcement_logic = "{{ example_config.compiled_in }}"


class LeanDebDeclaresNoPackages(Constraint):
    """CONSTRAINT-ID: CON-WD03

    `packaging/deb.toml` (the tetron-wm package) declares no `depends` and
    no `recommends`, so `apt install tetron-wm` pulls in nothing beyond what
    the binary links (WMDEB-006). Tools belong in `suggests`, or in the
    tetron-wm-desktop metapackage.
    ENFORCEMENT (reconcile.py): lean_deb.ok is true.
    """
    constraint_id = "CON-WD03"
    enforcement_logic = "{{ lean_deb.ok }}"


class DesktopIsAMetapackageOnTetronWm(Constraint):
    """CONSTRAINT-ID: CON-WD04

    `packaging/desktop.toml` is a metapackage (no files entry with src
    binary or asset) and its depends names tetron-wm (WMDEB-007).
    ENFORCEMENT (reconcile.py): desktop_deb.ok is true.
    """
    constraint_id = "CON-WD04"
    enforcement_logic = "{{ desktop_deb.ok }}"
