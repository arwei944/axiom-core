#!/usr/bin/env python3
"""Replace deprecated `Layer` references with `RuntimeTier` across Rust crates."""
import re
from pathlib import Path

# Root directory for crates
ROOT = Path(r"D:\work\trae\axiom-core-project\crates")

# Files to skip (the kernel source itself is supposedly already handled)
SKIP_PATTERNS = [
    # We keep kernel source intact per task, but still update kernel tests if needed
]

# Replacement rules: (pattern, replacement)
REPLACEMENTS = [
    # Import statements
    (r'use axiom_kernel::layer::Layer;', r'use axiom_kernel::layer::RuntimeTier;'),
    
    # Fully qualified paths
    (r'axiom_kernel::layer::Layer', r'axiom_kernel::layer::RuntimeTier'),
    (r'axiom_kernel::Layer', r'axiom_kernel::RuntimeTier'),
    (r'::axiom_kernel::Layer', r'::axiom_kernel::RuntimeTier'),
    
    # Type annotations and usages
    (r': Layer\b', r': RuntimeTier'),
    (r'\bpub layer: Layer\b', r'pub layer: RuntimeTier'),
    (r'\blayer: Layer\b', r'layer: RuntimeTier'),
    (r'\b_fn layer\(&self\) -> Layer', r'fn layer(&self) -> RuntimeTier'),
    (r'\bfrom: Layer\b', r'from: RuntimeTier'),
    (r'\bto: Layer\b', r'to: RuntimeTier'),
    (r'\b_layer: Layer\b', r'_layer: RuntimeTier'),
    
    # Enum variants
    (r'\bLayer::Exec\b', r'RuntimeTier::Exec'),
    (r'\bLayer::Oversight\b', r'RuntimeTier::Oversight'),
    (r'\bLayer::Agent\b', r'RuntimeTier::Agent'),
    (r'\bLayer::Validate\b', r'RuntimeTier::Validate'),
    
    # source_layer / target_layer fields
    (r'\bsource_layer: Layer\b', r'source_layer: RuntimeTier'),
    (r'\btarget_layer: Layer\b', r'target_layer: RuntimeTier'),
    
    # Function parameters
    (r'\(layer: Layer\)', r'(layer: RuntimeTier)'),
    (r'\(from: Layer,', r'(from: RuntimeTier,'),
    (r'\(to: Layer,', r'(to: RuntimeTier,'),
]

def should_process(path: Path) -> bool:
    return path.suffix == ".rs" and path.is_file()

def process_file(path: Path) -> bool:
    original = path.read_text(encoding="utf-8")
    content = original
    changed = False
    
    for pattern, replacement in REPLACEMENTS:
        new_content = re.sub(pattern, replacement, content)
        if new_content != content:
            changed = True
            content = new_content
    
    if changed:
        path.write_text(content, encoding="utf-8")
        return True
    return False

def main():
    processed = 0
    changed = 0
    for path in ROOT.rglob("*.rs"):
        if should_process(path):
            processed += 1
            if process_file(path):
                changed += 1
                print(f"Updated: {path.relative_to(ROOT)}")
    
    print(f"\nProcessed {processed} files, updated {changed} files.")

if __name__ == "__main__":
    main()
