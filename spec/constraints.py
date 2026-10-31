from libspec import Constraint


class PackagingTomlsValid(Constraint):
    """CONSTRAINT-ID: CON-WD01

    Every `packaging/*.toml` parses, has no `scripts` or `maintainer` key
    (the maintainer comes from the DEB_MAINTAINER repository variable), and
    every `files` entry has a `src` that exists in the repository or is the
    built binary, and a `dst` under usr/. Vacuously true before step 7 adds
    the first file.
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
