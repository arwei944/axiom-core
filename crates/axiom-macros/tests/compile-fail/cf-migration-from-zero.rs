// migration(from = 0) should fail because version must be >= 1
#[derive(Debug)]
struct BadMigration;

#[axiom_macros::migration(from = 0)]
impl axiom_kernel::version::Migration for BadMigration {
    fn migrate(&self, input: serde_json::Value) -> axiom_kernel::KernelResult<serde_json::Value> {
        Ok(input)
    }
}

fn main() {}
