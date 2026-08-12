# Toolbar SVG wire-up

Same pattern as IME #16:
1. Module + assets land in PR
2. `scripts/apply_toolbar_svg.py` replaces emoji buttons
3. Workflow `apply-toolbar-svg.yml` runs on this branch and pushes app.rs

After bot commit + CI green → squash merge.
