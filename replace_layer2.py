#!/usr/bin/env python3
"""Second pass: fix remaining Layer references."""
import re
from pathlib import Path

ROOT = Path(r"D:\work\trae\axiom-core-project\crates")

REPLACEMENTS = [
    # import lists: use axiom_kernel::{CellId, Layer, ...}
    (r'use axiom_kernel::\{([^}]*)\bLayer\b([^}]*)\};', 
     lambda m: m.group(0).replace('Layer', 'RuntimeTier').replace(', RuntimeTier', 'RuntimeTier')),
    
    # use axiom_kernel::layer::Layer, -> RuntimeTier,
    (r'use axiom_kernel::layer::Layer,', r'use axiom_kernel::layer::RuntimeTier,'),
    
    # Option<Layer>
    (r'Option<Layer\b', r'Option<RuntimeTier'),
    
    # HashMap<Layer, ...> etc
    (r'(?<![a-zA-Z_])(?:HashMap<|Option<|Box<|Vec<|Arc<)\s*Layer\b', 
     lambda m: m.group(0).replace('Layer', 'RuntimeTier')),
     
    # fn layer(&self) -> Layer
    (r'fn layer\(&self\)\s*->\s*Layer\b', r'fn layer(&self) -> RuntimeTier'),
    
    # pub use axiom_kernel::{CellId, Layer, ...}
    (r'pub use axiom_kernel::\{[^}]*Layer[^}]*\}', 
     lambda m: m.group(0).replace('Layer', 'RuntimeTier')),
     
    # use ... ::Layer, (in import lists) - but NOT CapabilityDimension::Layer
    (r'(?<!CapabilityDimension::)(?<!\w)(?:\s+Layer\s*,\s*\n?)', r'RuntimeTier,'),
]

def main():
    for path in ROOT.rglob("*.rs"):
        if not path.is_file():
            continue
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
            print(f"Updated: {path.relative_to(ROOT)}")

if __name__ == "__main__":
    main()
